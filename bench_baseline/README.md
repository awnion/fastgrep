# GNU grep baseline benchmark

Criterion benchmark for GNU grep, run on demand to produce reference numbers.

`BASELINE_GREP` env var must be set to the path of the grep binary.

## Quick run

```sh
BASELINE_GREP=/usr/bin/grep cargo bench --bench baseline_bench --features baseline
```

## Full run with results saved

```sh
BASELINE_GREP=/usr/bin/grep ./bench_baseline/run_baseline.sh "Apple M2 Max, 32 GB"
```

This runs the benchmark and writes results to [`baseline.md`](baseline.md).
