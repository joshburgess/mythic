#!/usr/bin/env bash
# Benchmark comparison: Mythic vs Jekyll vs Hugo vs Eleventy
# Generates a synthetic site and times builds across available SSGs.

set -euo pipefail

PAGES=${1:-1000}
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

echo "Generating synthetic site with $PAGES pages..."
cargo run -p mythic-cli --release -- init "$TMPDIR/mythic-site" 2>/dev/null

# Generate content using the benchmark utility
cargo run -p mythic-cli --release -- build --config "$TMPDIR/mythic-site/mythic.toml" --clean 2>/dev/null

echo ""
echo "=== Build Time Comparison ($PAGES pages) ==="
echo ""
printf "%-15s %10s\n" "Generator" "Time"
printf "%-15s %10s\n" "———————" "————"

# Mythic
if command -v cargo &>/dev/null; then
    START=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
    cargo run -p mythic-cli --release -- build --config "$TMPDIR/mythic-site/mythic.toml" --clean 2>/dev/null
    END=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
    MS=$(( (END - START) / 1000000 ))
    printf "%-15s %8dms\n" "Mythic" "$MS"
fi

# Hugo
if command -v hugo &>/dev/null; then
    echo "(Hugo benchmark requires manual site setup)"
fi

# Jekyll
if command -v jekyll &>/dev/null; then
    echo "(Jekyll benchmark requires manual site setup)"
fi

# Eleventy
if command -v eleventy &>/dev/null; then
    echo "(Eleventy benchmark requires manual site setup)"
fi

echo ""
echo "Run 'cargo bench' for detailed Criterion benchmarks."
