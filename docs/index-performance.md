# Index pipeline measurement

Measured locally on an Apple M2 Max (12 logical CPUs), macOS, with release builds
using Rust 1.99.0 and default thread counts. The baseline is
commit `1051421`; the comparison includes the index and shared search pipeline
fixes described in the v0.1.9 changelog.

The generated corpus contains 200 files with 1,000 lines each, totalling
14,068,018 bytes. One file contains `UniqueNeedleToken`. Output is discarded.
Both versions search identical files, alternating measurement order.

| Search mode | Before, median | After, median | Samples per version |
| --- | ---: | ---: | ---: |
| First search, including index construction | 291.777 ms | 44.571 ms | 6 |
| Search with an existing index | 7.680 ms | 7.381 ms | 20 |
| Search with `--no-index` | 7.621 ms | 7.423 ms | 20 |

Each first-search sample uses a fresh isolated cache directory. Filesystem pages
are not evicted: this measures a cold application index, not cold disk I/O.
The first-search improvement is about 6.5x on this corpus. The small differences
in the other modes should be treated as noise, not a speedup claim. Different
file sizes, contents, hardware, and thread counts can change these results.

Workers now extract trigrams from existing search buffers in parallel. A bounded
channel feeds an index collector, avoiding the old serial reread and extraction
pass after searching. Query planning checks metadata once and intersects sorted
postings without temporary hash sets.

To reproduce, provide release binaries built from the two revisions with the
same compiler and profile:

```sh
python3 benches/index_pipeline.py --before /path/to/old/fastgrep --after /path/to/new/fastgrep
```

The script uses only the Python standard library, creates a temporary corpus and
separate caches, enforces a 30-second timeout per search, and prints JSON results.
