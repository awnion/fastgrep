use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use regex::bytes::Regex;

use crate::cli::ResolvedConfig;

/// A compiled search pattern together with a deterministic cache key.
///
/// The cache key is derived from the raw pattern strings and all flags
/// that influence matching semantics (`-i`, `-v`, `-w`, `-F`).
pub struct CompiledPattern {
    pub regex: Regex,
    pub cache_key: String,
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
    /// assert!(pattern.regex.is_match(b"Hello World"));
    /// ```
    pub fn compile(config: &ResolvedConfig) -> Result<Self, regex::Error> {
        let combined = if config.patterns.len() == 1 {
            config.patterns[0].clone()
        } else {
            config.patterns.iter().map(|p| format!("(?:{p})")).collect::<Vec<_>>().join("|")
        };

        let pattern = if config.fixed_strings { regex::escape(&combined) } else { combined };

        let pattern = if config.word_regexp { format!(r"\b(?:{pattern})\b") } else { pattern };

        let regex = regex::bytes::RegexBuilder::new(&pattern)
            .case_insensitive(config.ignore_case)
            .unicode(false)
            .build()?;

        let cache_key = Self::make_cache_key(config);

        Ok(Self { regex, cache_key })
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
