#!/bin/sh
set -eu

output=${1:-/results}
workload=/src/scripts/profiling/workload.py
mkdir -p "$output"

{
    uname -a
    rustc --version
    perf --version
    git -C /opt/FlameGraph rev-parse HEAD
    printf 'RUSTFLAGS=%s\n' "$RUSTFLAGS"
} > "$output/environment.txt"

for mode in cold warm scan regex dense; do
    frequency=997
    case "$mode" in
        cold) repeats=8 ;;
        warm) repeats=200; frequency=9999 ;;
        scan) repeats=100 ;;
        regex) repeats=30 ;;
        dense) repeats=20 ;;
    esac
    python3 "$workload" run --mode "$mode" --repeat 10 > "$output/$mode.timing.json"
    perf stat -x , -e task-clock,context-switches,cpu-migrations,page-faults \
        -o "$output/$mode.stat.csv" -- \
        python3 "$workload" run --mode "$mode" --repeat "$repeats" \
        > "$output/$mode.stat-timing.json"
    perf record -q -e cpu-clock:u -F "$frequency" --call-graph fp \
        -o "$output/$mode.data" -- \
        python3 "$workload" run --mode "$mode" --repeat "$repeats" \
        > "$output/$mode.profile-timing.json" 2> "$output/$mode.record.log"
    perf report -i "$output/$mode.data" --stdio --no-children \
        --call-graph none --sort comm,dso,symbol --percent-limit 0.5 \
        > "$output/$mode.report.txt" 2> "$output/$mode.report.log"
    perf script -i "$output/$mode.data" 2> "$output/$mode.script.log" \
        | /opt/FlameGraph/stackcollapse-perf.pl > "$output/$mode.folded"
    /opt/FlameGraph/flamegraph.pl --hash --title "fastgrep: $mode (cpu-clock, user space)" \
        --countname nanoseconds "$output/$mode.folded" > "$output/$mode.svg"
    printf 'Finished %s\n' "$mode"
done
