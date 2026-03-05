# Changelog

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
