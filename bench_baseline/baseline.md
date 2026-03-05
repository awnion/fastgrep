# GNU grep baseline

- **Machine:** Apple M2 Max, 32 GB
- **GNU grep:** 3.11
- **Date:** 2025-03-05

Corpus: generated source-code-like Rust files (200 files × 5000 lines unless noted).

| Benchmark | Time |
|-----------|------|
| `-rn` literal sparse ("fn main") | 33.1 ms |
| `-rl` literal ("fn main") | 7.3 ms |
| `-rc` dense ("use ") | 84.0 ms |
| `-rni` case-insensitive ("error") | 73.2 ms |
| `-rn` regex (`impl\s+Drop`) | 76.4 ms |
| `-rn` very sparse ("SubscriptionManager") | 39.5 ms |
| single file (100k lines) | 5.5 ms |

## Scaling with file count

2000 lines per file.

| Files | Time |
|-------|------|
| 50 | 6.3 ms |
| 200 | 17.4 ms |
| 500 | 38.2 ms |

---

*Re-generate with: `./bench_baseline/run_baseline.sh "Apple M2 Max, 32 GB"`*
