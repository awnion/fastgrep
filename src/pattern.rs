use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use memchr::memmem::Finder;
use regex::bytes::Regex;

use crate::cli::ResolvedConfig;

/// A compiled search pattern together with a deterministic cache key.
///
/// For simple literal patterns (no regex metacharacters, no `-i`, no `-w`),
/// a SIMD-accelerated `memchr::memmem::Finder` is used instead of regex
/// for significantly faster matching.
pub struct CompiledPattern {
    pub regex: Regex,
    pub cache_key: String,
    /// Fast literal searcher, set when the pattern is a plain byte string.
    literal: Option<Finder<'static>>,
    /// SIMD-accelerated finder for the literal prefix of a regex pattern.
    /// Used for candidate filtering: find prefix with memmem, verify with regex.
    prefix: Option<Finder<'static>>,
}

impl CompiledPattern {
    /// Builds a [`CompiledPattern`] from the resolved configuration.
    ///
    /// Multiple `-e` patterns are combined with alternation (`|`).
    /// The `-F` flag escapes all regex metacharacters, and `-w` wraps
    /// the pattern in word-boundary anchors.
    ///
    /// # Errors
    ///
    /// Returns [`regex::Error`] if the resulting pattern is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use clap::Parser;
    /// use fastgrep::cli::Cli;
    /// use fastgrep::pattern::CompiledPattern;
    ///
    /// let cli = Cli::parse_from(["grep", "-i", "hello"]);
    /// let config = cli.resolve();
    /// let pattern = CompiledPattern::compile(&config).unwrap();
    /// assert!(pattern.is_match(b"Hello World"));
    /// ```
    pub fn compile(config: &ResolvedConfig) -> Result<Self, regex::Error> {
        let escaped: Vec<String> = if config.fixed_strings {
            config.patterns.iter().map(|p| regex::escape(p)).collect()
        } else {
            config.patterns.clone()
        };

        let pattern = if escaped.len() == 1 {
            escaped.into_iter().next().unwrap()
        } else {
            escaped.iter().map(|p| format!("(?:{p})")).collect::<Vec<_>>().join("|")
        };

        let pattern = if config.line_regexp {
            format!("^(?:{pattern})$")
        } else if config.word_regexp {
            format!(r"\b(?:{pattern})\b")
        } else {
            pattern
        };

        let regex = regex::bytes::RegexBuilder::new(&pattern)
            .case_insensitive(config.ignore_case)
            .unicode(false)
            .build()?;

        // Use fast literal path when possible: single pattern, no -i, no -w,
        // and no regex metacharacters (or -F is set)
        let literal = if config.patterns.len() == 1
            && !config.ignore_case
            && !config.word_regexp
            && !config.line_regexp
            && (config.fixed_strings || is_literal(&config.patterns[0]))
        {
            Some(Finder::new(config.patterns[0].as_bytes()).into_owned())
        } else {
            None
        };

        // Extract literal prefix for regex acceleration: use memmem to find
        // candidate positions, then verify with regex (skips most of the file).
        let prefix = if literal.is_none() && config.patterns.len() == 1 && !config.ignore_case {
            let raw = &config.patterns[0];
            let pfx = extract_literal_prefix(raw);
            if pfx.len() >= 2 { Some(Finder::new(pfx.as_bytes()).into_owned()) } else { None }
        } else {
            None
        };

        let cache_key = Self::make_cache_key(config);

        Ok(Self { regex, cache_key, literal, prefix })
    }

    /// Returns `true` if `haystack` matches this pattern.
    ///
    /// Uses SIMD-accelerated literal search when available, falling
    /// back to regex otherwise.
    #[inline]
    pub fn is_match(&self, haystack: &[u8]) -> bool {
        if let Some(ref finder) = self.literal {
            finder.find(haystack).is_some()
        } else {
            self.regex.is_match(haystack)
        }
    }

    /// Returns the literal `Finder` if this pattern is a plain byte string.
    #[inline]
    pub fn literal_finder(&self) -> Option<&Finder<'_>> {
        self.literal.as_ref()
    }

    /// Returns a `Finder` for the literal prefix of a regex pattern.
    /// Used for candidate filtering in whole-buffer search.
    #[inline]
    pub fn prefix_finder(&self) -> Option<&Finder<'_>> {
        self.prefix.as_ref()
    }

    /// Extracts the set of required trigrams from the pattern's literal
    /// or prefix bytes. Returns an empty vec when no trigrams can be
    /// extracted (pure regex), meaning no file filtering is possible.
    pub fn required_trigrams(&self) -> Vec<[u8; 3]> {
        let needle = self
            .literal
            .as_ref()
            .map(|f| f.needle())
            .or_else(|| self.prefix.as_ref().map(|f| f.needle()));
        match needle {
            Some(bytes) if bytes.len() >= 3 => {
                let mut seen = std::collections::HashSet::new();
                for w in bytes.windows(3) {
                    seen.insert([w[0], w[1], w[2]]);
                }
                seen.into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    /// Produces a hex-encoded hash that uniquely identifies this
    /// pattern + flag combination for cache lookups.
    fn make_cache_key(config: &ResolvedConfig) -> String {
        let mut hasher = DefaultHasher::new();
        config.patterns.hash(&mut hasher);
        config.ignore_case.hash(&mut hasher);
        config.invert_match.hash(&mut hasher);
        config.word_regexp.hash(&mut hasher);
        config.fixed_strings.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

/// Returns `true` if the pattern contains no regex metacharacters.
fn is_literal(pattern: &str) -> bool {
    !pattern.contains(['.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '^', '$', '\\'])
}

/// Extracts the longest literal prefix from a regex pattern.
///
/// Walks the pattern character by character, stopping at the first
/// regex metacharacter. Handles simple escape sequences like `\\.`.
///
/// Returns empty string for patterns with top-level alternation (`|`)
/// since the prefix is not common to all alternatives.
fn extract_literal_prefix(pattern: &str) -> String {
    // If pattern has top-level alternation, no single prefix is safe
    if has_top_level_alternation(pattern) {
        return String::new();
    }

    let mut prefix = String::new();
    let mut chars = pattern.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            // Metacharacters end the literal prefix
            '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' => {
                break;
            }
            '\\' => {
                chars.next(); // consume backslash
                match chars.peek() {
                    // Literal escapes
                    Some(
                        &ec @ ('.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
                        | '^' | '$' | '\\'),
                    ) => {
                        prefix.push(ec);
                        chars.next();
                    }
                    // All other escapes (\d, \w, \b, etc.) are not literal
                    _ => break,
                }
            }
            _ => {
                prefix.push(c);
                chars.next();
            }
        }
    }

    prefix
}

/// Returns `true` if the pattern contains `|` outside of any grouping.
fn has_top_level_alternation(pattern: &str) -> bool {
    let mut depth: u32 = 0;
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next(); // skip escaped char
            }
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}
