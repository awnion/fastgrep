#!/usr/bin/env bash
set -euo pipefail

# Usage: ./bench_baseline/run_baseline.sh "Apple M2 Max, 32GB"
#
# Runs the baseline benchmark (external grep binary) and saves
# results to bench_baseline/baseline.md.
#
# BASELINE_GREP must be set to the grep binary path:
#   BASELINE_GREP=/usr/bin/grep ./bench_baseline/run_baseline.sh "Linux x86_64, 16GB"

MACHINE="${1:?Usage: $0 \"Machine description\"}"
GREP_BIN="${BASELINE_GREP:?BASELINE_GREP env var must be set (e.g. BASELINE_GREP=/usr/bin/grep)}"
GREP_VERSION=$("$GREP_BIN" --version 2>&1 | head -1 || echo "unknown")
DATE=$(date +%Y-%m-%d)
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
OUTPUT="$SCRIPT_DIR/baseline.md"

echo "Running baseline benchmark with: $GREP_BIN"
echo "Machine: $MACHINE"
echo ""

# Run criterion benchmark, capture output
BENCH_OUTPUT=$(cargo bench --bench baseline_bench --features baseline 2>&1)

# Parse criterion output: extract "time: [X Y Z]" median values
# Format: "group/name   time:   [low est high]"
parse_time() {
    local group="$1"
    echo "$BENCH_OUTPUT" | grep -A2 "$group/grep" | grep "time:" | sed -E 's/.*\[.*[[:space:]]+(.*)[[:space:]]+.*/\1/' | head -1
}

# Extract results
rn_sparse=$(parse_time "baseline_rn_literal_sparse")
rl_literal=$(parse_time "baseline_rl_literal")
rc_dense=$(parse_time "baseline_rc_dense")
rni_case=$(parse_time "baseline_rni_case_insensitive")
rn_regex=$(parse_time "baseline_rn_regex_prefix")
rn_very_sparse=$(parse_time "baseline_rn_very_sparse")
single_file=$(parse_time "baseline_single_file_100k_lines")

scale_50=$(echo "$BENCH_OUTPUT" | grep -A2 "baseline_scaling/grep/50" | grep "time:" | sed -E 's/.*\[.*[[:space:]]+(.*)[[:space:]]+.*/\1/' | head -1)
scale_200=$(echo "$BENCH_OUTPUT" | grep -A2 "baseline_scaling/grep/200" | grep "time:" | sed -E 's/.*\[.*[[:space:]]+(.*)[[:space:]]+.*/\1/' | head -1)
scale_500=$(echo "$BENCH_OUTPUT" | grep -A2 "baseline_scaling/grep/500" | grep "time:" | sed -E 's/.*\[.*[[:space:]]+(.*)[[:space:]]+.*/\1/' | head -1)

cat > "$OUTPUT" <<EOF
# GNU grep baseline

- **Machine:** $MACHINE
- **GNU grep:** $GREP_VERSION
- **Date:** $DATE

Corpus: generated source-code-like Rust files (200 files × 5000 lines unless noted).

| Benchmark | Time |
|-----------|------|
| \`-rn\` literal sparse ("fn main") | $rn_sparse |
| \`-rl\` literal ("fn main") | $rl_literal |
| \`-rc\` dense ("use ") | $rc_dense |
| \`-rni\` case-insensitive ("error") | $rni_case |
| \`-rn\` regex (\`impl\s+Drop\`) | $rn_regex |
| \`-rn\` very sparse ("SubscriptionManager") | $rn_very_sparse |
| single file (100k lines) | $single_file |

## Scaling with file count

2000 lines per file.

| Files | Time |
|-------|------|
| 50 | $scale_50 |
| 200 | $scale_200 |
| 500 | $scale_500 |

---

*Re-generate with: \`./bench_baseline/run_baseline.sh "$MACHINE"\`*
EOF

echo ""
echo "Saved to $OUTPUT"
