# GNU grep baseline

- **Machine:** Apple M2 Max, 12 logical CPUs, 32 GB
- **OS:** macOS 27.0.1 (26A434)
- **GNU grep:** 3.12 (`ggrep`, Homebrew)
- **Last run:** 2026-10-05
- **Locale:** `LC_ALL=C`
- **Harness:** Criterion 0.8.2 at commit `5094623`

Corpus: generated Rust-like files (200 files × 5,000 lines unless noted).
Scaling uses 2,000 lines per file; the single-file workload uses 100,000 lines.
Times are Criterion's central estimates, including subprocess startup and
captured output, rounded to two decimal places. Main workloads use 100 samples;
scaling uses 10. The filesystem cache is warm.

| Benchmark | GNU grep |
| --- | ---: |
| `-rn` literal sparse (`fn main`) | 32.99 ms |
| `-rl` literal (`fn main`) | 7.12 ms |
| `-rc` dense (`use `) | 84.75 ms |
| `-rni` case-insensitive (`error`) | 82.57 ms |
| `-rn` regex (`impl\s+Drop`) | 73.43 ms |
| `-rn` very sparse (`SubscriptionManager`) | 39.35 ms |
| Single file (100k lines) | 5.21 ms |

| Files | GNU grep |
| --- | ---: |
| 50 | 6.17 ms |
| 200 | 17.07 ms |
| 500 | 38.49 ms |

See the [full fastgrep comparison](../docs/benchmarks/README.md)
for confidence intervals, methodology, and both fastgrep modes.
The [raw GNU grep log](../docs/benchmarks/gnu-grep.log) and
[numeric estimates](../docs/benchmarks/results.json) record this run.

Re-generate with:

```sh
LC_ALL=C BASELINE_GREP=ggrep ./bench_baseline/run_baseline.sh "Apple M2 Max, 32 GB"
```

On Linux, use the path to GNU grep instead of `ggrep`.
