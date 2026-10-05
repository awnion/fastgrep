# Linux perf in Docker / OrbStack

Run from the repository root. Docker needs to be running. These commands keep
source mounted read-only and place the corpus/build in the Linux filesystem.
They do not change host kernel settings or require a fully privileged container.

```sh
mkdir -p target/profiling
docker build -t fastgrep-profiler:local -f scripts/profiling/Dockerfile scripts/profiling
docker run -d --name fastgrep-profiler --init \
  --cap-add PERFMON --cap-add SYS_PTRACE --security-opt seccomp=unconfined \
  -v "$PWD:/src:ro" -v "$PWD/target/profiling:/results" \
  fastgrep-profiler:local sleep infinity
docker exec fastgrep-profiler sh -c '
  mkdir -p /work/repo
  tar -C /src --exclude=target --exclude=.git -cf - . | tar -C /work/repo -xf -
  cd /work/repo
  cargo build --locked --release --bin fastgrep
  python3 /src/scripts/profiling/workload.py generate > /results/corpus.json
  sh /src/scripts/profiling/profile.sh
'
```

The image enables release debug symbols and frame pointers but retains the
repository's optimization level. It pins Rust and the FlameGraph revision.
`profile.sh` writes timings, perf stat CSV, perf data, reports, folded stacks and
interactive SVG flamegraphs. The application cache is reset for cold runs only.
`perf.data` and logs are kept out of Git by the existing `/target` ignore rule.

Hardware counters may be unavailable inside a VM. The script intentionally uses
software CPU-clock events. A kernel-inclusive profile may also lack symbols;
the default graphs cover user space. Folded weights are CPU-clock nanoseconds.

## Isolated experiments

The saved bitmap patch is a prototype against `f5657a8`, not a production change.
Build and test it only in a separate source copy:

```sh
docker exec fastgrep-profiler sh -c '
  mkdir -p /work/bin
  cp /work/target/release/fastgrep /work/bin/baseline
  cd /work/repo
  CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_TARGET_DIR=/work/target-opt3 \
    cargo build --locked --release --bin fastgrep
  cp /work/target-opt3/release/fastgrep /work/bin/opt3
  cp -a /work/repo /work/bitmap
  cd /work/bitmap
  patch -p1 < /src/scripts/profiling/trigram-bitmap.patch
  CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_TARGET_DIR=/work/target-opt3 \
    cargo build --locked --release --bin fastgrep
  cp /work/target-opt3/release/fastgrep /work/bin/bitmap
  CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_TARGET_DIR=/work/target-opt3 \
    cargo test --locked --release
  python3 /src/scripts/profiling/compare.py > /results/comparison.json
'
```

Run timing comparisons without concurrent builds or other profiling jobs.
`workload.py run --threads N --mode MODE` can measure a chosen worker count.
Individual searches have a 60-second timeout. The corpus defaults to 512 files
and 4,000 lines each; adjust `generate --files N --lines N` using a fresh directory.

Stop the container after collecting results:

```sh
docker stop fastgrep-profiler
```

Use `docker start fastgrep-profiler` to inspect the retained perf data with the
same Linux binaries, or `docker rm fastgrep-profiler` to discard its build/corpus.
