# Changelog

## v0.1.5

### Improvements

- Trigram index now carries a format version; incompatible or corrupted indexes are automatically deleted and rebuilt
- `--version` output shows the index format version

### Testing

- Integration tests split into 9 focused modules (basic matching, flags, files/recursive, stdin, edge cases, binary, only-matching, context, regression)
- Added `rstest` for parametrized test cases, reducing boilerplate

## v0.1.4

### New features

- `-o` / `--only-matching` — print only the matched parts of a line, each on its own line
- `-A NUM` / `--after-context` — print NUM lines of trailing context after each match
- `-B NUM` / `--before-context` — print NUM lines of leading context before each match
- `-C NUM` / `--context` — print NUM lines of context before and after each match
- Group separators (`--`) between non-contiguous context blocks (GNU grep compatible)
- Context lines use `-` separator instead of `:` (GNU grep compatible)

### Improvements

- `--version` now shows git commit hash (or `release` for crates.io installs), crates.io link, and feature list
- `--help` updated with AI-agent-optimized messaging

### Testing

- 24 new integration tests (125 total)

## v0.1.3

### Packaging

- Release binaries are now distributed as `.tar.gz` archives
- Added one-line install commands for Linux (musl) and macOS in README

## v0.1.2

### CI/CD

- Reusable workflow architecture (`_lint.yml`, `_test.yml`, `_build.yml`)
- Release pipeline: lint → test → build → smoke-test → publish (crates.io + GitHub Releases)
- Nightly toolchain for `rustfmt` and `clippy` in CI
- Release tests run under `--release` profile
- Added `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` build targets
- Parallel lint jobs (format, clippy, docs run concurrently)
- GitHub Releases with platform binaries and changelog

## v0.1.1

### New features

- **File size limit** — files larger than 100 MiB are skipped by default with a machine-readable warning to stderr. Override with `FASTGREP_NO_LIMIT=1` or `--max-file-size=<BYTES>`. Prevents grep from hanging on accidental large binaries in the repo.

### Performance

- New parallel directory walker with cooperative work-stealing across 2–4 threads
- Performance optimizations in search pipeline

### Testing

- Expanded integration test suite (101 tests covering flags, edge cases, binary detection, color output)

### Documentation

- Added install instructions and usage examples to README
- Added `ENVIRONMENT.md` with full reference of env vars and CLI flags
- Added `AI_AGENT_GREP_USECASES.md` documenting common grep patterns used by AI coding agents
- Documented all differences from GNU grep behaviour

## v0.1.0

Initial release.

- GNU grep-compatible interface (`-r`, `-i`, `-n`, `-l`, `-c`, `-v`, `-w`, `-E`, `-F`, `--include`, `--exclude`, `--color`)
- Parallel search across all CPU threads
- SIMD-accelerated literal and prefix search via `memchr::memmem`
- Lazy trigram index with automatic invalidation
- Line truncation (`--max-line-len`, default 15000)
