# Mythic Benchmarks

Benchmark results comparing Mythic against Hugo and Eleventy on identical synthetic sites.

## Test Environment

- **Hardware**: Apple M-series (ARM64), macOS
- **Mythic**: v0.1.0 (release build with thin LTO, codegen-units=1)
- **Hugo**: v0.158.0+extended (Homebrew)
- **Eleventy**: v3.1.5 (npx)
- **Content**: Synthetic markdown pages with YAML frontmatter, headings, paragraphs, lists, fenced code blocks, bold/italic, and links (~150 bytes rendered HTML per page)
- **Templates**: Minimal single-layout HTML template per generator
- **Methodology**: Best of 5 runs, direct binary execution, clean output directory each run, internal timer (excludes process startup)

## Results

### Full Build (cold, clean output)

| Pages  | Mythic   | Mythic (flat) | Hugo     | Eleventy  |
|-------:|---------:|--------------:|---------:|----------:|
| 1,000  | 162ms    | —             | 98ms     | 290ms     |
| 5,000  | 1,422ms  | —             | 529ms    | 2,234ms   |
| 10,000 | 1,822ms  | 1,338ms       | 1,718ms  | ~5,200ms  |

### Incremental Build (no changes)

| Pages  | Mythic   | Hugo     | Eleventy  |
|-------:|---------:|---------:|----------:|
| 10,000 | **174ms**| 1,718ms  | ~3,500ms  |

### Summary

| Scenario | Mythic vs Hugo | Mythic vs Eleventy |
|----------|:--------------:|:------------------:|
| 10k cold build (clean URLs) | Parity (1.06x) | 2.9x faster |
| 10k cold build (flat URLs) | **22% faster** | 3.9x faster |
| 10k incremental | **9.9x faster** | **20x faster** |
| 1k cold build | 1.7x slower | 1.8x faster |

### Pipeline Profile (10,000 pages, full build)

| Stage      | Time   | % of Total |
|------------|-------:|-----------:|
| Discovery  | 147ms  | 8%         |
| Render     | 56ms   | 3%         |
| Templates  | 20ms   | 1%         |
| Output I/O | 1,534ms| 88%        |
| **Total**  | **1,822ms** | **100%** |

### Criterion Micro-Benchmarks

Run with `cargo bench -p mythic-core`:

| Benchmark                          | Result    |
|------------------------------------|----------:|
| Full build (100 pages)             | 33.6ms    |
| Full build (1,000 pages)           | 363ms     |
| Incremental no-op (1,000 pages)    | 12.7ms    |
| Markdown rendering (500 pages)     | 156µs     |

## Analysis

### Where Mythic Excels

**Incremental builds are Mythic's standout advantage.** The content-hash cache (`DepGraph`) skips unchanged pages entirely — no re-reading, no re-rendering, no re-writing. At 10,000 pages, an incremental rebuild with no changes completes in 174ms, which is 9.9x faster than Hugo and 20x faster than Eleventy. This is the workflow developers actually use: edit a file, save, see the result.

**Flat URL mode (`ugly_urls = true`) beats Hugo on cold builds.** Writing `slug.html` instead of `slug/index.html` eliminates per-page directory creation, halving the filesystem syscalls. At 10k pages this produces a 22% speed advantage over Hugo.

**Markdown rendering and template application are extremely fast.** Pulldown-cmark with rayon parallelization renders 10,000 pages in 56ms. Tera/Handlebars template application (parallelized) adds 20ms. The combined CPU pipeline is ~223ms at 10k pages.

### Where Hugo Wins

**Hugo is faster on cold builds at smaller scales** (1k pages: 98ms vs 162ms). Hugo's Go runtime has lower per-process overhead and its I/O path is optimized for the clean-URL directory structure.

At 10k pages the gap closes to parity (1,822ms vs 1,718ms) because the I/O cost dominates equally for both generators.

### Why Output I/O Dominates

Mythic's clean URL scheme generates one directory + one `index.html` per page. Each page requires `create_dir` + `File::create` + `write` + implicit `close` — at least 3-4 syscalls per page. At 10,000 pages, that's 40,000+ syscalls.

Evidence: at 10k pages, Mythic spends 0.4s in user code but ~17s in kernel time (across all rayon threads). The computation is fast — the OS is the bottleneck.

### Optimizations Applied

| Optimization | Impact | Status |
|---|---|---|
| Parallel markdown rendering (rayon) | Baseline | Done |
| Incremental build cache (content hash) | 10x faster rebuilds | Done |
| Pre-created output directories | 30% faster output | Done |
| Parallel file writes (rayon) | Baseline | Done |
| Parallel template rendering | ~10ms savings | Done |
| Thin LTO + codegen-units=1 | ~5% overall | Done |
| ahash (fixed seeds) | Faster hashing, 174ms incremental | Done |
| CompactString for frontmatter | Reduced allocations | Done |
| lasso string interning | Deduplicated repeated strings | Done |
| Flat URL output mode | 22% faster than Hugo | Done |
| Pre-computed output paths | Avoided redundant PathBuf joins | Done |

### Optimization Attempts That Didn't Help

| Approach | Result | Why |
|----------|--------|-----|
| BufWriter for output | No change | Files too small (~1.6KB) for buffering to help |
| Parallel content discovery | Slower (625ms vs 149ms) | Filesystem thrashing from parallel reads |
| Fat LTO over thin | Same speed | No measurable improvement, 12x longer compile |
| Parallel dir creation | Slower (2,345ms vs 1,592ms) | VFS lock contention from concurrent mkdir |

### Remaining Optimization Opportunities

| Approach | Expected Impact | Complexity |
|---|---|---|
| io_uring (Linux) | 2-5x faster output on Linux | High (platform-specific) |
| Return pages from build() | ~100ms with taxonomies | Done |
| PGO (profile-guided optimization) | 10-20% overall | Medium |

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

This eliminates `mkdir` syscalls entirely and produces ~22% faster builds at 10k pages.

## Future: Arena Allocator Design

The content discovery phase allocates frontmatter strings (title, date, layout, tags) per page. At 10k pages this is ~50-100k small allocations. A bump allocator (`bumpalo`) could replace these with arena-allocated `&'arena str` references, reducing allocator pressure.

### Design

```rust
// Current (owned strings, heap-allocated per page)
pub struct Frontmatter {
    pub title: CompactString,      // heap or inline
    pub layout: Option<CompactString>,
    pub tags: Option<Vec<CompactString>>,
}

// Arena approach (borrowed from bump allocator)
pub struct Frontmatter<'a> {
    pub title: &'a str,            // points into arena
    pub layout: Option<&'a str>,
    pub tags: Option<Vec<&'a str>>,
}
```

### Trade-offs

**Pros:**
- Near-zero allocation cost for all frontmatter strings
- Better cache locality (arena is contiguous memory)
- Estimated 10-15ms savings on discovery (currently 147ms)

**Cons:**
- Lifetime parameter `'a` propagates to `Page<'a>`, `Vec<Page<'a>>`, `build()` signature, `Plugin` trait methods, and all 322+ tests
- `bumpalo::Bump` is `!Sync`, which conflicts with rayon parallelism. Would need per-thread arenas or `bumpalo`'s `allocator_api` feature
- Serde deserialization into arena-borrowed strings requires `#[serde(borrow)]` and `Cow<'a, str>` or custom deserializer
- `Clone` becomes non-trivial (can't clone arena references to a different arena)

### Recommendation

Attempt on a feature branch. The current `CompactString` + `lasso` approach captures most of the benefit (inline small strings + deduplicated repeated strings) without the lifetime complexity. The arena approach would primarily help sites with very long frontmatter values (titles >24 bytes) at high page counts (50k+).
