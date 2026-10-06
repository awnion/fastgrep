# fastgrep

[![Crates.io](https://img.shields.io/crates/v/fastgrep)](https://crates.io/crates/fastgrep)
[![docs.rs](https://img.shields.io/docsrs/fastgrep)](https://docs.rs/fastgrep)
[![Crates.io downloads](https://img.shields.io/crates/d/fastgrep)](https://crates.io/crates/fastgrep)

A drop-in replacement for GNU grep that is parallel by default, builds a lazy trigram index, and is designed from the ground up to be **AI-native first**.

## Why

LLM agents and AI-powered dev tools run grep thousands of times per session. Every millisecond matters at that scale. fastgrep combines SIMD-accelerated literal search, multi-threaded parallelism, and a lazy trigram index to accelerate recursive searches. Performance depends on the query and corpus; see the [measured comparison](#benchmarks). No upfront indexing required — the trigram index warms up on the first run and is invalidated automatically when files change (mtime + size check).

## Install

### With Cargo

```sh
cargo install fastgrep
```

Cargo installs both `fastgrep` and `grep`. To install only the explicit name:

```sh
cargo install fastgrep --bin fastgrep
fastgrep -rn 'TODO' src/
```

To use it as your default grep:

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

Each archive contains both `grep` and `fastgrep`. All binaries are available on the [GitHub releases page](https://github.com/awnion/fastgrep/releases).

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

# JSON Lines output for machine parsing
grep --json -rn 'TODO' src/

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

Last benchmark run: **2026-10-05**. Measured on Apple M2 Max (12 logical CPUs), 32 GB,
macOS 27.0.1: fastgrep 0.1.9 (Rust 1.99.0, release build) versus GNU grep 3.12,
with `LC_ALL=C`. Criterion benchmarks use a generated Rust-like corpus
(200 files × 5,000 lines, unless noted). Timings include process startup and
captured output; indexed searches use a warm index and exclude its construction.

```text
+---------------------------------------+-----------+------------+----------+---------+
| Workload                              | fastgrep* | --no-index | GNU grep | GNU/fg* |
+---------------------------------------+-----------+------------+----------+---------+
| -rn "fn main" (sparse)                |   8.82 ms |    7.31 ms | 32.99 ms |   3.74x |
| -rl "fn main"                         |   7.50 ms |    5.88 ms |  7.12 ms |   0.95x |
| -rc "use " (dense)                    |   8.55 ms |    7.64 ms | 84.75 ms |   9.91x |
| -rni "error"                          |  37.49 ms |          - | 82.57 ms |   2.20x |
| -rn impl\s+Drop                       |  45.24 ms |   35.31 ms | 73.43 ms |   1.62x |
| -rn SubscriptionManager (very sparse) |   5.41 ms |    6.30 ms | 39.35 ms |   7.28x |
| Single file (100k lines)              |   3.70 ms |          - |  5.21 ms |   1.41x |
+---------------------------------------+-----------+------------+----------+---------+
```

Scaling (2,000 lines per file):

```text
+-------+-----------+------------+----------+---------+
| Files | fastgrep* | --no-index | GNU grep | GNU/fg* |
+-------+-----------+------------+----------+---------+
| 50    |   4.17 ms |    3.43 ms |  6.17 ms |   1.48x |
| 200   |   7.02 ms |    5.70 ms | 17.07 ms |   2.43x |
| 500   |  12.50 ms |    9.65 ms | 38.49 ms |   3.08x |
+-------+-----------+------------+----------+---------+
```

`fastgrep*` uses a warm index where applicable; `--no-index` is fastgrep with
indexing disabled. Scaling uses 2,000 lines per file. The ratio is GNU grep
time / fastgrep's default time; values below 1 mean GNU grep is faster. Case-insensitive and
single-file searches do not use the index; their separate `--no-index` variants
are not measured by the suite.

On this corpus, `--no-index` is often faster when the index cannot skip files.
Results depend on the pattern, hardware, locale, and cache state. See the
[full benchmark report](docs/benchmarks/README.md) for methodology,
confidence intervals, raw logs, and reproduction commands, or the
[GNU grep baseline](bench_baseline/baseline.md).

For a separate before/after measurement of index construction, see
[Index pipeline measurement](docs/index-performance.md).
Linux `perf` profiles and optimization experiments are recorded in
[the OrbStack profiling report](docs/profiling/2026-10-05/README.md).

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

GNU grep has no indexing. fastgrep lazily builds a trigram index during the first recursive search, using the same file buffers as the search workers. It stores the index under the OS cache directory (`~/.cache/fastgrep/trigram/` on Linux, `~/Library/Caches/fastgrep/trigram/` on macOS).

Subsequent searches skip only known, unchanged files that provably cannot match. New, previously excluded, binary, and changed files are searched normally. File metadata is checked once per index plan; when more than 10% of indexed files are stale, the current search rebuilds and replaces the index. New files remain searchable even before a rebuild includes them.

Filtering is disabled for `-v`, `-L`, and `-c`, since these modes need results from files without pattern matches. Version 2 indexes use absolute file paths; older caches are rebuilt automatically. Disable indexing with `--no-index`.

## Build

Requires Rust 1.99 or newer. The repository selects the stable toolchain.

```sh
cargo build --release
```

The binaries are at `target/release/grep` and `target/release/fastgrep`.

## Test

On macOS, install GNU grep with `brew install grep` first. Integration tests
compare against `ggrep` on macOS and `/usr/bin/grep` on Linux.

```sh
# integration tests (compared against GNU grep)
cargo test

# also check the upcoming compiler
cargo +nightly test

# benchmarks (fastgrep only)
LC_ALL=C cargo bench --locked --bench grep_bench

# baseline benchmark (GNU grep, on demand)
LC_ALL=C BASELINE_GREP=ggrep cargo bench --locked --bench baseline_bench --features baseline
```

CI runs the full test suite on stable and nightly on Ubuntu 24.04 and 26.04
(amd64 and arm64) and macOS 26 (Apple Silicon). Linux artifacts are built on
Ubuntu 24.04. A separate matrix then checks both GNU and musl archives on
Ubuntu 24.04, Ubuntu 26.04, and Debian 13 on both architectures. These are the
two latest Ubuntu LTS releases and the latest stable Debian release. macOS
archives support Apple Silicon only. Binary compatibility checks run after
builds in ordinary CI and gate publishing in release CI, including help,
version (matched against the release tag for releases), and a basic search.
Formatting, Clippy, documentation, and ordinary CI builds use nightly. Release
artifacts and crates.io publishing use stable. Formatting requires nightly:
`cargo +nightly fmt --all`.

## GNU grep compatibility

Most common GNU grep flags are supported. See [GNU_GREP_COMPAT.md](GNU_GREP_COMPAT.md) for the remaining unimplemented flags.

## Environment variables

See [ENVIRONMENT.md](ENVIRONMENT.md) for the full list of environment variables and CLI flags.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
