# Linux perf profile: f5657a8

Profiling ran in an ARM64 Debian 12 Docker container on OrbStack, with 12 CPUs,
Rust 1.99.0 and the OrbStack 7.0.14 kernel. Production sources were not changed.
The release profile retained `opt-level = "s"`, LTO and one codegen unit. Debug
information and frame pointers were enabled; stripping was disabled.

The generated Rust-like corpus follows the patterns in `benches/corpus.rs`:
512 files, 4,000 lines each, 173,597,020 bytes (165.6 MiB). Corpus, cache and build
artifacts lived in the container filesystem, not a macOS bind mount. Output went
to `/dev/null`. Cold means an empty trigram cache, not cold filesystem pages.

## Measurements

Median milliseconds, ten runs per cell, alternating variant order:

| Workload | Current `s` profile | `opt-level=3` | `opt-level=3` + bitmap prototype |
| --- | ---: | ---: | ---: |
| Cold index, `-rl SubscriptionManager` | 322.28 | 273.25 | 124.14 |
| Warm index, same query | 7.65 | 7.74 | 8.30 |
| No index, same query | 11.15 | 11.41 | 11.28 |
| Regex count, `--no-index -rcE 'impl\s+Drop'` | 13.43 | 17.66 | 15.47 |
| Dense output, `--no-index -rn 'let var_'` | 22.94 | 20.50 | 22.16 |

The bitmap experiment is a separate source copy in the container. Its patch is
saved in [trigram-bitmap.patch](../../../scripts/profiling/trigram-bitmap.patch).
It replaces per-file `HashSet<[u8; 3]>` deduplication with a direct-address bitmap
of all 2^24 keys, using 2 MiB of scratch memory per concurrently processed file.
The combined experiment is 2.60x faster on cold indexing; the bitmap's isolated
gain relative to the `opt-level=3` build is 2.20x.

This is not a ready production change. One separate cold-search RSS measurement
was 14,272 KiB for the baseline and 55,300 KiB for the prototype. A production
implementation should consider reusable worker scratch space and a sparse or
small-file strategy. The prototype passed all 235 integration tests and 9 doc
tests in Linux release mode. It has not been validated across all platforms or
against adversarial/high-entropy workloads.

`opt-level=3` alone is not uniformly beneficial: regex counting became slower
in this run, and the binary's text section increased from 1,690,669 to 1,979,905
bytes (17%). Do not switch the entire release profile based only on cold indexing.

## Flamegraphs and actionable hotspots

Widths represent sampled **user-space CPU time**, not elapsed time. Percentages
below are self samples unless explicitly described as combined. Short tasks and
inlining limit attribution. Hardware PMU events (`cycles`, `instructions`) were
not supported by the VM, so no IPC, branch-miss or cache-miss claims are made.

1. **Trigram deduplication: highest priority.**
   [Cold flamegraph](cold.svg), [perf report](cold.report.txt).
   Hash-table insertion (39.9%), SipHash `write` (30.1%), and `hash_one` (23.8%)
   account for about 94% of sampled user CPU time. This directly identifies
   `FileSnapshot::extract` in `src/trigram.rs`. The bitmap experiment confirms
   that avoiding hashing each overlapping three-byte window has substantial value.
   [Prototype flamegraph](bitmap.svg) shows the changed distribution after hashing
   is removed. Allocation/zeroing, index collection and B-tree comparisons then
   deserve attention; the prototype also has substantially more context switches.

2. **Unnecessary binary scan in positive file-list searches.**
   [No-index flamegraph](scan.svg), [perf report](scan.report.txt).
   `is_binary` uses 44.4% of sampled user CPU, and literal searching uses 45.9%.
   Both walk the same file. Investigate bypassing binary detection for positive
   `-l` when binary classification cannot affect the output. Preserve `-I`,
   inversion and binary semantics; add differential GNU grep tests before changing
   this. A general removal of binary detection would be incorrect.

3. **Per-invocation index decoding.**
   [Warm flamegraph](warm.svg), [perf report](warm.report.txt).
   Index `load` (22.8%) and bitcode's serde decoder (12.3%) together occupy 35.2%
   of sampled user CPU; binary scanning contributes another 17.5%. The cache file
   is 1,931,611 bytes, decoded into vectors and a B-tree on every CLI invocation.
   Compare native bitcode encoding or a flat representation before considering
   a more complex persistent service. These CPU percentages are not equivalent
   to potential wall-time savings: startup, metadata operations and scheduling
   are also significant on this short workload.

4. **Counting regex matches pays for line-number traversal.**
   [Regex flamegraph](regex.svg), [perf report](regex.report.txt).
   `count_matches` accounts for 46.8% of sampled user CPU, prefix searching 27.2%,
   and binary detection 15.2%. Its prefix path advances `LineCursor` through every
   intervening line even though counts do not need line numbers. A count-specific
   cursor locating only candidate line boundaries is worth measuring.

5. **Dense output spends time assembling buffers.**
   [Dense flamegraph](dense.svg), [perf report](dense.report.txt).
   `Vec<u8>::extend_from_slice` accounts for 26.9%, literal search 14.3%, integer
   formatting 6.4%, and `write_line_match` 4.9%. Investigate reserving/assembling
   each output record with fewer small appends. This workload discards output;
   real terminal or pipe throughput can dominate instead.

## Thread-count experiment

Baseline binary, medians of 20 runs per cell, separate sequential batches:

| Workload | `-j1` | `-j4` | `-j12` |
| --- | ---: | ---: | ---: |
| Warm index | 10.10 ms | 5.82 ms | 6.55 ms |
| No-index literal | 22.80 ms | 11.82 ms | 12.64 ms |
| Regex count | 75.14 ms | 21.78 ms | 17.92 ms |
| Dense output | 131.96 ms | 41.97 ms | 34.95 ms |

This supports testing adaptive worker counts, especially after index filtering,
rather than lowering the thread count for all searches. Separate batches show
normal VM timing variation and should not be compared directly to the variant
table as a regression test.

## Collection and limitations

- `perf stat`: task-clock, context switches, migrations, page faults.
- `perf record`: `cpu-clock:u`, frame-pointer call graphs; 997 Hz for the longer
  workloads and 9,999 Hz for warm search to improve visibility of short tasks.
- No lost samples were reported. An additional kernel-inclusive probe could not
  resolve kernel symbols; the published flamegraphs therefore show user space.
- The Python repetition harness is included in perf events and flamegraphs;
  uninstrumented wall timings measure each grep subprocess, excluding corpus
  generation and cache deletion. Do not treat profiler-run elapsed times as
  benchmark timings.
- All comparisons use the same corpus, VM and compiler. These are synthetic,
  local results, not a guarantee for real repositories or physical Linux hosts.

[metrics.json](metrics.json) retains the raw wall-time samples, thread sweep,
perf stat output, environment and top sampled symbols. Raw `perf.data`, folded
stacks and logs remain under the ignored `target/profiling/` directory. The stopped
`fastgrep-profiler-f5657a8` container retains the matching binaries and symbols.

See [profiling instructions](../../../scripts/profiling/README.md) to reproduce.
