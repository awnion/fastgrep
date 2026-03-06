# fastgrep

[![Crates.io](https://img.shields.io/crates/v/fastgrep)](https://crates.io/crates/fastgrep)
[![docs.rs](https://img.shields.io/docsrs/fastgrep)](https://docs.rs/fastgrep)
[![Crates.io downloads](https://img.shields.io/crates/d/fastgrep)](https://crates.io/crates/fastgrep)

A drop-in replacement for GNU grep that is parallel by default, builds a lazy trigram index, and is designed from the ground up to be **AI-native first**.

## Why

LLM agents and AI-powered dev tools run grep thousands of times per session. Every millisecond matters at that scale. fastgrep combines SIMD-accelerated literal search, multi-threaded parallelism, and a lazy trigram index to be **2–12x faster than GNU grep** across common workloads. No upfront indexing required — the trigram index warms up on the first run and is invalidated automatically when files change (mtime + size check).

## Install

### With Cargo

```sh
cargo install fastgrep
```

The installed binary is called `grep`. To use it as your default grep:

```sh
# option 1: alias (add to .bashrc / .zshrc)
alias grep="$(cargo bin-dir 2>/dev/null || echo ~/.cargo/bin)/grep"

# option 2: ensure ~/.cargo/bin is before /usr/bin in PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

### From binary releases

**Linux (static musl binary, works everywhere including Docker):**

```sh
curl -fsSL --retry 3 https://github.com/awnion/fastgrep/releases/latest/download/grep-x86_64-unknown-linux-musl.tar.gz | tar xz -C /usr/local/bin
```

**macOS (Apple Silicon):**

```sh
curl -fsSL --retry 3 https://github.com/awnion/fastgrep/releases/latest/download/grep-aarch64-apple-darwin.tar.gz | tar xz -C /usr/local/bin
```

All binaries are available on the [GitHub releases page](https://github.com/awnion/fastgrep/releases).

## Usage

```sh
# exactly like GNU grep
grep -rn 'TODO' src/

# search only in specific file types
grep -rn 'class User' --include='*.py' .

# case-insensitive search
grep -rni 'error' src/

# fixed string (no regex interpretation)
grep -rFn 'Vec<Box<dyn Error>>' --include='*.rs' .

# list files containing matches
grep -rl 'migration' src/

# count matches per file
grep -rc 'unwrap()' --include='*.rs' .

# show only matched parts
grep -o -E '[0-9]+\.[0-9]+\.[0-9]+' Cargo.toml

# context: 2 lines after each match
grep -rn -A2 'fn main' --include='*.rs' .

# context: 1 line before and after
grep -rn -C1 'panic!' --include='*.rs' .

# second run is faster (trigram index cache hit)
grep -rn 'TODO' src/

# disable trigram index
grep --no-index -rn 'pattern' src/

# control parallelism
grep -j4 -r 'error' .

# pipe from stdin
cat log.txt | grep 'FATAL'
```

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

> GNU grep baseline measured on Apple M2 Max, 32 GB. See [`bench_baseline/baseline.md`](bench_baseline/baseline.md).

## Differences from GNU grep

fastgrep intentionally departs from GNU grep behaviour in several places. Every deviation is motivated by the same goal: **make recursive search safe and fast for AI agents that can't babysit a hung process**.

### File size limit (default 100 MiB)

```
WARNING: 1 file(s) skipped due to size limit:
  - ./data/model.bin (2300.0 MB)

These files may cause grep to hang. To search them anyway, re-run with:
  FASTGREP_NO_LIMIT=1 grep ...
Or adjust the threshold: --max-file-size=<BYTES> (current: 100 MiB)
```

GNU grep will happily read a 2 GB binary blob line by line, taking minutes or effectively hanging. This is the single biggest pain point for AI agents — the agent is blocked, the user is waiting, and the tool has no way to know it should stop.

fastgrep skips files larger than 100 MiB by default and reports them to stderr. The warning is machine-readable: an agent can parse it, add `--exclude`, and retry. Override with `FASTGREP_NO_LIMIT=1` or `--max-file-size=<BYTES>`.

### Line truncation (default 15000 bytes)

GNU grep outputs lines of any length. In practice, minified JS bundles or serialized data can produce single lines of 10+ MB that flood an agent's context window with noise.

fastgrep truncates lines beyond `--max-line-len` (default 15000). Set to 0 to disable.

### Parallel by default

GNU grep is single-threaded. fastgrep uses all available CPU threads by default (`-j0`). This changes the **output order** — files are printed in whichever order workers finish, not in filesystem walk order. For AI agents this doesn't matter (they parse `file:line:content` tuples), but it means `diff <(grep ...) <(fastgrep ...)` may differ in line order.

### Trigram index

GNU grep has no indexing. fastgrep lazily builds a trigram index on first recursive search and caches it at `~/.cache/fastgrep/trigram/`. Subsequent searches skip files that provably can't match. The index is invalidated automatically (mtime + size check). Disable with `--no-index`.

## Build

```sh
cargo build --release
```

The binary is at `target/release/grep`.

## Test

```sh
# integration tests (compared against GNU grep)
cargo test

# benchmarks (fastgrep only)
cargo bench

# baseline benchmark (GNU grep, on demand)
cargo bench --bench baseline_bench --features baseline
```

## GNU grep compatibility

Most common GNU grep flags are supported. See [GNU_GREP_COMPAT.md](GNU_GREP_COMPAT.md) for the remaining unimplemented flags.

## Environment variables

See [ENVIRONMENT.md](ENVIRONMENT.md) for the full list of environment variables and CLI flags.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
