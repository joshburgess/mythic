# Mythic Benchmarks

Benchmark results comparing Mythic against Hugo and Eleventy on identical synthetic sites.

## Test Environment

- **Hardware**: Apple M-series (ARM64), macOS
- **Mythic**: v0.1.0 (release build with thin LTO, codegen-units=1)
- **Hugo**: v0.158.0+extended (Homebrew)
- **Eleventy**: v3.1.5 (npx)
- **Content**: Synthetic markdown pages with YAML frontmatter, headings, paragraphs, lists, fenced code blocks, bold/italic, and links (~500 words per page)
- **Templates**: Minimal single-layout HTML template per generator
- **Methodology**: Best of 5 runs, direct binary execution, clean output directory each run

## Results

### Full Build (cold, clean output)

Mythic supports two URL modes:
- **Clean URLs** (default): each page outputs to `slug/index.html`, producing pretty URLs like `/about/`
- **Flat URLs** (`ugly_urls = true`): each page outputs to `slug.html`, skipping directory creation for faster I/O

| Pages  | Mythic (clean URLs) | Mythic (flat URLs) | Hugo     | Eleventy  |
|-------:|--------------------:|-------------------:|---------:|----------:|
| 1,000  | 150ms               | 125ms              | 171ms    | 300ms     |
| 5,000  | 740ms               | 614ms              | 851ms    | 1,510ms   |
| 10,000 | 1,614ms             | 1,261ms            | 2,925ms  | 3,860ms   |

### Incremental Build (no changes)

Mythic's incremental cache skips rendering, templating, and writing for unchanged pages entirely. Hugo and Eleventy re-process all pages on every build.

| Pages  | Mythic    | Hugo     | Eleventy  |
|-------:|----------:|---------:|----------:|
| 1,000  | **10ms**  | 171ms    | ~300ms    |
| 5,000  | **58ms**  | 851ms    | ~1,510ms  |
| 10,000 | **125ms** | 2,925ms  | ~3,860ms  |

### Summary

| Scenario | Mythic vs Hugo | Mythic vs Eleventy |
|----------|:--------------:|:------------------:|
| 1k cold build | **12% faster** | 2.0x faster |
| 5k cold build | **13% faster** | 2.0x faster |
| 10k cold build | **45% faster** | 2.4x faster |
| 10k flat URLs | **57% faster** | 3.1x faster |
| 1k incremental | **17x faster** | **30x faster** |
| 5k incremental | **15x faster** | **26x faster** |
| 10k incremental | **23x faster** | **31x faster** |

### Pipeline Profile (10,000 pages, full build)

| Stage      | Time   | % of Total |
|------------|-------:|-----------:|
| Discovery  | 121ms  | 7%         |
| Render     | 161ms  | 10%        |
| Templates  | 5ms    | <1%        |
| Output I/O | 1,348ms| 83%        |
| **Total**  | **1,637ms** | **100%** |

### Criterion Micro-Benchmarks

Run with `cargo bench -p mythic-core`:

| Benchmark                          | Result    |
|------------------------------------|----------:|
| Full build (100 pages)             | 25.0ms    |
| Full build (1,000 pages)           | 283ms     |
| Incremental no-op (1,000 pages)    | 12.7ms    |
| Markdown rendering (500 pages)     | 111µs     |

## Analysis

### Where Mythic Excels

**Mythic is faster than Hugo on cold builds at every scale tested.** At 1k pages Mythic is 12% faster; at 10k pages 45% faster. With flat URLs, the advantage grows to 57% at 10k pages.

**Incremental builds are Mythic's standout advantage.** The content-hash cache skips unchanged pages entirely: no re-reading, no re-rendering, no re-templating, no re-writing. At 10,000 pages, an incremental rebuild with no changes completes in 125ms, which is 23x faster than Hugo and 31x faster than Eleventy. This is the workflow developers actually use: edit a file, save, see the result.

**Template rendering is near-zero cost.** Collections (page lists, sections) are registered as lazy Tera functions, so per-page template rendering doesn't need to clone large data structures. At 10k pages, the template phase takes 5ms.

**Markdown rendering is extremely fast.** Pulldown-cmark with rayon parallelization renders 10,000 pages in 161ms.

**Config and template changes are detected automatically.** The incremental cache tracks a combined hash of the config file and all template files. When either changes, all pages are rebuilt. No manual cache clearing needed.

### Why Output I/O Dominates

Mythic's clean URL scheme generates one directory + one `index.html` per page. Each page requires `create_dir` + `File::create` + `write` + implicit `close`, totaling at least 3 to 4 syscalls per page. At 10,000 pages, that's 40,000+ syscalls. The I/O phase accounts for 83% of build time.

**Flat URLs** (`ugly_urls = true`) eliminate per-page directory creation, cutting I/O syscalls roughly in half. This produces the fastest builds: 1,261ms at 10k pages (57% faster than Hugo).

### Optimizations Applied

| Optimization | Impact | Status |
|---|---|---|
| Lazy template collections (Tera functions) | Eliminated O(n²) template overhead | Done |
| True incremental: skip render + template for unchanged | 23x faster incremental at 10k | Done |
| Config + template hash invalidation | Correct rebuilds without hurting perf | Done |
| Parallel markdown rendering (rayon) | Baseline | Done |
| Incremental build cache (content hash) | Skip unchanged pages entirely | Done |
| Pre-built shared template context | Avoids repeated serialization | Done |
| Pre-created output directories | 30% faster output | Done |
| Parallel file writes (rayon) | Baseline | Done |
| Parallel template rendering | Baseline | Done |
| Thin LTO + codegen-units=1 | ~5% overall | Done |
| ahash (fixed seeds) | Faster hashing | Done |
| CompactString for frontmatter | Reduced allocations | Done |
| lasso string interning | Deduplicated repeated strings | Done |
| Flat URL output mode | Eliminates per-page mkdir | Done |
| Pre-computed output paths | Avoided redundant PathBuf joins | Done |
| Post-build skip on incremental no-op | Avoids search/diff/feed regen | Done |

### Remaining Optimization Opportunities

| Approach | Expected Impact | Complexity |
|---|---|---|
| io_uring (Linux) | 2-5x faster output on Linux | High (platform-specific) |
| PGO (profile-guided optimization) | 10-20% overall | Medium |
| Arena allocator for frontmatter (bumpalo) | ~10-15ms on discovery | High (lifetime propagation) |

## Reproducing Benchmarks

### Quick Comparison

```bash
# Build with profiling
cargo run --release -p mythic-cli -- build --config mythic.toml --profile
```

### Criterion Suite

```bash
cargo bench -p mythic-core
```

Results are saved to `target/criterion/` with HTML reports.

### Cross-Generator Comparison

```bash
# Install competitors
brew install hugo
npm install -g @11ty/eleventy

# Run the comparison script
./scripts/benchmark-comparison.sh 1000
```

### Generating Large Test Sites

```rust
use mythic_core::bench_utils::generate_site;
use std::path::Path;

// Generate a 10,000 page site with seed 42 for reproducibility
generate_site(Path::new("/tmp/bench-site"), 10_000, 42);
```

Each generated page includes ~500 words of realistic markdown with headings, paragraphs, lists, code blocks, links, bold/italic text, and randomized tags.

### Flat URL Mode

To benchmark with flat URLs (no per-page directories):

```toml
# mythic.toml
ugly_urls = true
```

This eliminates `mkdir` syscalls entirely and produces the fastest builds at all scales.
