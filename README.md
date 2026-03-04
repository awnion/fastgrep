# fastgrep

A drop-in replacement for GNU grep that is parallel by default, builds a lazy cache in `~/.cache/fastgrep/`, and is designed from the ground up to be **AI-native first**.

## Why

LLM agents and AI-powered dev tools run grep thousands of times per session. Every millisecond matters at that scale. fastgrep makes repeated searches near-instant by caching results and invalidating them only when files change (mtime + size check). No upfront indexing required — the cache warms up naturally as you work.

## Key ideas

- **GNU grep interface** — same flags you already know (`-r`, `-i`, `-n`, `-l`, `-c`, `-v`, `-w`, `-E`, `-F`, `--include`, `--exclude`, `--color`)
- **Parallel by default** — uses all available CPU threads out of the box
- **Lazy cache** — first query runs at normal speed; every subsequent query for the same pattern is served from `~/.cache/fastgrep/v1/` with mtime-based invalidation
- **AI-native first** — optimised for the access patterns of LLM agents: high query volume, repeated patterns, large codebases

## Usage

```sh
# exactly like GNU grep
grep -rn 'TODO' src/

# second run is faster (cache hit)
grep -rn 'TODO' src/

# disable cache when you don't need it
grep --no-cache 'pattern' file.txt

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
