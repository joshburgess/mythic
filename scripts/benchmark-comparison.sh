#!/usr/bin/env bash
# Benchmark comparison: Mythic vs Hugo vs Eleventy
# Generates identical synthetic sites and times builds across available SSGs.
#
# Usage: ./scripts/benchmark-comparison.sh [PAGES]
#   PAGES defaults to 1000

set -euo pipefail

PAGES=${1:-1000}
RUNS=5
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

echo "=== Benchmark: $PAGES pages, best of $RUNS runs ==="
echo ""

# --- Generate identical content ---
generate_content() {
    local dir=$1
    for i in $(seq 1 "$PAGES"); do
        local m=$(printf '%02d' $(( (i % 12) + 1 )))
        local d=$(printf '%02d' $(( (i % 28) + 1 )))
        cat > "$dir/post-${i}.md" << EOF
---
title: "Post ${i}"
date: "2024-${m}-${d}"
---
# Post ${i}

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam.

## Section One

Paragraph with **bold** and *italic* and a [link](https://example.com). More text to simulate a realistic blog post.

- Item one with some description text
- Item two with more details here
- Item three concluding the list

## Section Two

Another paragraph with additional content.

\`\`\`rust
fn main() {
    println!("Hello from post ${i}");
    let x = vec![1, 2, 3];
    for item in &x {
        println!("{}", item);
    }
}
\`\`\`

Final paragraph wrapping up post number ${i}.
EOF
    done
}

best_of() {
    local best=999999
    for ms in "$@"; do
        if [ "$ms" -lt "$best" ]; then best=$ms; fi
    done
    echo "$best"
}

# --- Mythic ---
echo "Setting up Mythic..."
MYTHIC_DIR="$TMPDIR/mythic"
mkdir -p "$MYTHIC_DIR/content" "$MYTHIC_DIR/templates"
cat > "$MYTHIC_DIR/mythic.toml" << 'EOF'
title = "Benchmark Site"
base_url = "http://localhost:3000"
EOF
cat > "$MYTHIC_DIR/templates/default.html" << 'EOF'
<!DOCTYPE html><html><head><title>{{ page.title }}</title></head><body><article>{{ content | safe }}</article></body></html>
EOF
generate_content "$MYTHIC_DIR/content"

echo "Building Mythic release binary..."
cargo build --release -p mythic-cli 2>/dev/null
MYTHIC_BIN="$(cargo metadata --format-version 1 2>/dev/null | python3 -c 'import sys,json; print(json.load(sys.stdin)["target_directory"])')/release/mythic"

echo ""
echo "--- Cold Build ---"
echo ""

# Mythic cold
printf "%-15s" "Mythic"
mythic_times=()
for i in $(seq 1 $RUNS); do
    rm -rf "$MYTHIC_DIR/public"
    output=$("$MYTHIC_BIN" build --config "$MYTHIC_DIR/mythic.toml" 2>&1)
    ms=$(echo "$output" | grep -oE '[0-9]+ms' | head -1 | sed 's/ms//')
    mythic_times+=("$ms")
    printf " %5dms" "$ms"
done
best=$(best_of "${mythic_times[@]}")
printf "  → best: %dms\n" "$best"

# Hugo
if command -v hugo &>/dev/null; then
    HUGO_DIR="$TMPDIR/hugo"
    mkdir -p "$HUGO_DIR/layouts/_default" "$HUGO_DIR/content/posts"
    cat > "$HUGO_DIR/hugo.toml" << 'EOF'
baseURL = "http://localhost"
title = "Benchmark Site"
EOF
    cat > "$HUGO_DIR/layouts/_default/baseof.html" << 'LAYOUT'
<!DOCTYPE html><html><head><title>{{ .Title }}</title></head><body>{{ block "main" . }}{{ end }}</body></html>
LAYOUT
    cat > "$HUGO_DIR/layouts/_default/single.html" << 'LAYOUT'
{{ define "main" }}<article><h1>{{ .Title }}</h1>{{ .Content }}</article>{{ end }}
LAYOUT
    cat > "$HUGO_DIR/layouts/_default/list.html" << 'LAYOUT'
{{ define "main" }}{{ range .Pages }}<a href="{{ .Permalink }}">{{ .Title }}</a>{{ end }}{{ end }}
LAYOUT
    generate_content "$HUGO_DIR/content/posts"

    printf "%-15s" "Hugo"
    hugo_times=()
    for i in $(seq 1 $RUNS); do
        rm -rf "$HUGO_DIR/public"
        output=$(hugo --source "$HUGO_DIR" 2>&1)
        ms=$(echo "$output" | grep -oE '[0-9]+ ms' | head -1 | sed 's/ ms//')
        hugo_times+=("$ms")
        printf " %5dms" "$ms"
    done
    best=$(best_of "${hugo_times[@]}")
    printf "  → best: %dms\n" "$best"
fi

# Eleventy
if npx @11ty/eleventy --version &>/dev/null 2>&1; then
    ELEVENTY_DIR="$TMPDIR/eleventy"
    mkdir -p "$ELEVENTY_DIR/content" "$ELEVENTY_DIR/_includes"
    cat > "$ELEVENTY_DIR/.eleventy.js" << 'EOF'
module.exports = function(eleventyConfig) {
  return { dir: { input: "content", output: "_site", includes: "../_includes" } };
};
EOF
    cat > "$ELEVENTY_DIR/_includes/default.njk" << 'EOF'
<!DOCTYPE html><html><head><title>{{ title }}</title></head><body><article>{{ content | safe }}</article></body></html>
EOF
    cat > "$ELEVENTY_DIR/content/content.json" << 'EOF'
{ "layout": "default.njk" }
EOF
    cat > "$ELEVENTY_DIR/package.json" << 'EOF'
{"private":true,"dependencies":{"@11ty/eleventy":"^3.1.5"}}
EOF
    generate_content "$ELEVENTY_DIR/content"
    (cd "$ELEVENTY_DIR" && npm install --silent 2>/dev/null)

    printf "%-15s" "Eleventy"
    eleventy_times=()
    for i in $(seq 1 $RUNS); do
        rm -rf "$ELEVENTY_DIR/_site"
        output=$(cd "$ELEVENTY_DIR" && npx @11ty/eleventy --quiet 2>&1)
        secs=$(echo "$output" | grep -oE '[0-9]+\.[0-9]+ seconds' | head -1 | sed 's/ seconds//')
        if [ -n "$secs" ]; then
            ms=$(python3 -c "print(int(float('$secs') * 1000))")
        else
            ms="ERR"
        fi
        eleventy_times+=("$ms")
        printf " %5dms" "$ms"
    done
    best=$(best_of "${eleventy_times[@]}")
    printf "  → best: %dms\n" "$best"
fi

# Mythic incremental
echo ""
echo "--- Incremental (no changes) ---"
echo ""
rm -rf "$MYTHIC_DIR/public"
"$MYTHIC_BIN" build --config "$MYTHIC_DIR/mythic.toml" >/dev/null 2>&1

printf "%-15s" "Mythic"
inc_times=()
for i in $(seq 1 $RUNS); do
    output=$("$MYTHIC_BIN" build --config "$MYTHIC_DIR/mythic.toml" 2>&1)
    ms=$(echo "$output" | grep -oE '[0-9]+ms' | head -1 | sed 's/ms//')
    inc_times+=("$ms")
    printf " %5dms" "$ms"
done
best=$(best_of "${inc_times[@]}")
printf "  → best: %dms\n" "$best"

echo ""
echo "--- Pipeline Profile ---"
echo ""
rm -rf "$MYTHIC_DIR/public"
"$MYTHIC_BIN" build --config "$MYTHIC_DIR/mythic.toml" --profile 2>&1

echo ""
echo "Run 'cargo bench -p mythic-core' for Criterion micro-benchmarks."
