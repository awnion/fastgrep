# fastgrep

[![Crates.io](https://img.shields.io/crates/v/fastgrep)](https://crates.io/crates/fastgrep)
[![docs.rs](https://img.shields.io/docsrs/fastgrep)](https://docs.rs/fastgrep)
[![Crates.io downloads](https://img.shields.io/crates/d/fastgrep)](https://crates.io/crates/fastgrep)

A drop-in replacement for GNU grep that is parallel by default, builds a lazy cache in `~/.cache/fastgrep/`, and is designed from the ground up to be **AI-native first**.

## Why

LLM agents and AI-powered dev tools run grep thousands of times per session. Every millisecond matters at that scale. fastgrep combines SIMD-accelerated literal search, multi-threaded parallelism, and a lazy trigram index to be **2–12x faster than GNU grep** across common workloads. No upfront indexing required — the trigram index warms up on the first run and is invalidated automatically when files change (mtime + size check).

## How it works

### Search pipeline

```
Pattern → Trigram Index → Walker → Thread Pool → Searcher → Output
```

1. **Pattern compilation** — the pattern is analyzed and compiled into the fastest available representation:
   - **Pure literal** — if the pattern has no regex metacharacters, it goes straight to SIMD-accelerated `memchr::memmem` (whole-buffer search, no per-line overhead)
   - **Prefix-accelerated regex** — if the pattern starts with a literal prefix (e.g. `impl\s+Drop` → prefix `impl`), the prefix is searched with SIMD first, and the full regex only runs on candidate lines
   - **Full regex** — fallback for complex patterns, case-insensitive mode, or multiple `-e` patterns

2. **Trigram index filtering** — before any file is opened, fastgrep checks a persistent trigram index to skip files that cannot possibly contain the pattern:
   - Every file's content is broken into 3-byte windows (trigrams) and stored in an inverted index
   - At query time, the pattern's trigrams are extracted and intersected — only files containing _all_ required trigrams become candidates
   - The index is built lazily on the first run and cached at `~/.cache/fastgrep/trigram/`
   - Invalidation is automatic: files are checked by mtime + size; if >10% are stale, the index is rebuilt
   - For very sparse patterns (e.g. a unique class name in 20 out of 200 files), this gives a measurable speedup by skipping 90% of I/O

3. **Parallel search** — candidate files are distributed across a thread pool (all CPUs by default). Large files (>4 MiB) are further split into chunks and searched in parallel within a single file.

4. **Streaming output** — results are written directly to stdout as they are found (no buffering of all matches in memory).

### Key ideas

- **GNU grep interface** — same flags you already know (`-r`, `-i`, `-n`, `-l`, `-c`, `-v`, `-w`, `-E`, `-F`, `--include`, `--exclude`, `--color`)
- **Parallel by default** — uses all available CPU threads out of the box
- **Trigram index** — persistent inverted index for sub-linear file filtering; no upfront indexing required, warms up lazily
- **SIMD literals** — `memchr::memmem` for literal patterns and literal prefixes of regex patterns, avoiding the regex engine on 90%+ of searches
- **AI-native first** — optimised for the access patterns of LLM agents: high query volume, repeated patterns, large codebases

## Benchmarks

Criterion benchmarks on a generated corpus (200 files × 5000 lines each).

| Benchmark                                   | fastgrep | GNU grep | Speedup  |
| ------------------------------------------- | -------- | -------- | -------- |
| `-rn` literal sparse (`"fn main"`)          | 7.6 ms   | 33.1 ms  | **4.4x** |
| `-rl` literal (`"fn main"`)                 | 6.7 ms   | 7.3 ms   | **1.1x** |
| `-rc` dense (`"use "`)                      | 6.8 ms   | 84.0 ms  | **12x**  |
| `-rni` case-insensitive (`"error"`)         | 35.6 ms  | 73.2 ms  | **2.1x** |
| `-rn` regex (`impl\s+Drop`)                 | 8.1 ms   | 76.4 ms  | **9.4x** |
| `-rn` very sparse (`"SubscriptionManager"`) | 5.5 ms   | 39.5 ms  | **7.2x** |
| single file (100k lines)                    | 3.5 ms   | 5.5 ms   | **1.6x** |

Scaling with file count:

| Files | fastgrep | GNU grep |
| ----- | -------- | -------- |
| 50    | 4.5 ms   | 6.3 ms   |
| 200   | 7.9 ms   | 17.4 ms  |
| 500   | 13.8 ms  | 38.2 ms  |

fastgrep scales ~2x better than GNU grep as file count grows.

## Usage

```sh
# exactly like GNU grep
grep -rn 'TODO' src/

# second run is faster (cache hit)
grep -rn 'TODO' src/

# disable trigram index
grep --no-index -rn 'pattern' src/

# control parallelism
grep -j4 -r 'error' .
```

## Build

```sh
cargo build --release
```

The binary is at `target/release/grep`.

## Test

```sh
# integration tests (compared against GNU grep)
cargo test

# benchmarks (fastgrep vs GNU grep via criterion)
cargo bench
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
