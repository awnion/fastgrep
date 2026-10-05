# GNU grep baseline benchmark

Criterion benchmark for GNU grep, run on demand to produce reference numbers.

`BASELINE_GREP` env var must be set to the path of the grep binary.
On macOS, use Homebrew's `ggrep`; `/usr/bin/grep` is BSD grep.

## Quick run

```sh
LC_ALL=C BASELINE_GREP=ggrep cargo bench --locked --bench baseline_bench --features baseline
```

## Full run with results saved

```sh
LC_ALL=C BASELINE_GREP=ggrep ./bench_baseline/run_baseline.sh "Apple M2 Max, 32 GB"
```

This runs the benchmark and writes results to [`baseline.md`](baseline.md).

On Linux, substitute the path to GNU grep (typically `/usr/bin/grep`).
Check `"$BASELINE_GREP" --version` before running. Use the same locale for both
the GNU grep and fastgrep benchmarks.

The latest comparison, including fastgrep with a warm index and `--no-index`,
is in [the benchmark report](../docs/benchmarks/README.md).
That report includes the original Criterion logs and numeric estimates.
