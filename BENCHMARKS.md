# Mythic Benchmarks

Benchmark results comparing Mythic against Hugo and Eleventy on identical synthetic sites.

## Test Environment

- **Hardware**: Apple M-series (ARM64), macOS
- **Mythic**: v0.1.0 (release build, `cargo build --release`)
- **Hugo**: v0.158.0+extended (Homebrew)
- **Eleventy**: v3.1.5 (npx)
- **Content**: Synthetic markdown pages with YAML frontmatter, headings, paragraphs, lists, fenced code blocks, bold/italic, and links (~150 bytes rendered HTML per page)
- **Templates**: Minimal single-layout HTML template per generator
- **Methodology**: Best of 5 runs, direct binary execution (no cargo overhead), clean output directory each run

## Results

### Full Build (cold, clean output)

| Pages  | Mythic   | Hugo     | Eleventy  | Mythic vs Hugo | Mythic vs Eleventy |
|-------:|---------:|---------:|----------:|:--------------:|:------------------:|
| 1,000  | 290ms    | 150ms    | 634ms     | 1.9x slower    | 2.2x faster        |
| 5,000  | 1,422ms  | 529ms    | 2,234ms   | 2.7x slower    | 1.6x faster        |
| 10,000 | 3,002ms  | 1,800ms  | 5,152ms   | 1.7x slower    | 1.7x faster        |

### Incremental Build (no changes)

| Pages  | Mythic   | Hugo     | Eleventy  | Hugo vs Mythic | Eleventy vs Mythic |
|-------:|---------:|---------:|----------:|:--------------:|:------------------:|
| 10,000 | 201ms    | 1,772ms  | 3,483ms   | 8.8x slower    | 17x slower         |

### Pipeline Profile (10,000 pages, full build)

| Stage      | Time   | % of Total |
|------------|-------:|-----------:|
| Discovery  | 149ms  | 8%         |
| Render     | 48ms   | 3%         |
| Templates  | 17ms   | 1%         |
| Output I/O | 1,572ms| 88%        |
| **Total**  | **1,787ms** | **100%** |

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

**Incremental builds are Mythic's standout advantage.** The content-hash cache (`DepGraph`) skips unchanged pages entirely — no re-reading, no re-rendering, no re-writing. At 10,000 pages, an incremental rebuild with no changes completes in 201ms, which is 8.8x faster than Hugo and 17x faster than Eleventy. This is the workflow developers actually use: edit a file, save, see the result.

**Markdown rendering and template application are extremely fast.** Pulldown-cmark with rayon parallelization renders 10,000 pages in 48ms. Tera template application adds 17ms. The combined render+template pipeline is ~65ms at 10k pages — competitive with any SSG.

### Where Hugo Wins

**Hugo is faster on cold builds** by roughly 1.7-2.7x. The gap is almost entirely in file output I/O. Writing 10,000 files (creating directories + writing index.html for each page) accounts for 88% of Mythic's build time. Hugo's Go runtime appears to handle high-volume filesystem operations with less syscall overhead.

Evidence: at 10k pages, Mythic spends 0.36s in user code but 18s in kernel syscalls (across all cores). The CPU computation is fast — the OS is the bottleneck.

### Why the I/O Gap Exists

Mythic's clean URL scheme generates one directory + one `index.html` per page:
```
public/posts/post-1/index.html
public/posts/post-2/index.html
...
```

Each page requires:
1. `create_dir_all()` — multiple stat + mkdir syscalls
2. `File::create()` — open syscall
3. `write()` — write syscall
4. Implicit `close()` on drop

At 10,000 pages, that's 40,000+ syscalls minimum. The actual data written (39MB) is trivial — a single `write()` of 39MB would complete in <50ms on any modern SSD.

### Optimization Attempts

| Approach | Result | Why |
|----------|--------|-----|
| Pre-create dirs, then parallel writes | **Best** (1,572ms) | Avoids dir contention during writes |
| Parallel dir+write combined | Worse (2,345ms) | `create_dir_all` contention between threads (34s system time) |
| BufWriter | No change | Files too small (~1.6KB) for buffering to help |
| Parallel content discovery (rayon) | Worse (625ms vs 149ms) | Filesystem thrashing from parallel reads |
| HashSet dedup for dirs | Marginal | Reduces redundant calls but syscall cost dominates |

### Future Optimization Opportunities

- **io_uring (Linux)**: Batch file operations into a single kernel submission. Could reduce 40k syscalls to a handful of batched operations.
- **Pre-allocated file pools**: Open file descriptors in bulk before writing.
- **Flat output with redirect map**: Write files without per-page directories, use server-side rewrites. Eliminates mkdir entirely.
- **Memory-mapped output**: Use `memmap2` for large files to avoid explicit write syscalls.
- **Content-addressed output**: Write to a content-addressed store, then symlink/hardlink to output paths. Deduplicates identical pages.

## Reproducing Benchmarks

### Quick Comparison

```bash
# Generate a synthetic site
cargo run --release -p mythic-cli -- init bench-site

# Build with profiling
cargo run --release -p mythic-cli -- build --config bench-site/mythic.toml --profile
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

The `mythic_core::bench_utils::generate_site()` function creates deterministic synthetic sites at any scale:

```rust
use mythic_core::bench_utils::generate_site;
use std::path::Path;

// Generate a 10,000 page site with seed 42 for reproducibility
generate_site(Path::new("/tmp/bench-site"), 10_000, 42);
```

Each generated page includes ~500 words of realistic markdown with headings, paragraphs, lists, code blocks, links, bold/italic text, and randomized tags.
