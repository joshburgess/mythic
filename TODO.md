# Mythic — Next Steps

## Hygiene (quick wins)

- [x] Fix all clippy warnings (15 warnings)
- [x] Run `cargo fmt --all` to fix formatting drift
- [x] Add LICENSE file (MIT)
- [x] Add CHANGELOG.md

## User Experience

- [ ] **Colored CLI output** — use `colored` or `owo-colors` for errors (red), warnings (yellow), success (green), and build summaries
- [ ] **`mythic new` command** — `mythic new post "My Title"` creates `content/posts/my-title.md` with frontmatter scaffold (title, date, draft: true)
- [ ] **`--verbose` flag** — show each file being processed, template applied, output written
- [ ] **Friendly template errors** — catch Tera/Handlebars errors and reformat with the template filename, line number, and a suggestion instead of raw stack traces
- [ ] **Config validation** — warn on unrecognized keys in `mythic.toml` (catches typos like `titl` or `base-url`). Use `serde(deny_unknown_fields)` or a validation pass.
- [ ] **Bundle starters in binary** — embed starter templates via `include_dir` or `rust-embed` so `mythic init --template blog` works from an installed binary, not just from the workspace

## Missing Features

- [ ] **Pagination** — paginate taxonomy term pages and section listing pages. Config: `paginate = 10` per taxonomy or in `_dir.yaml`. Generate `/tags/rust/page/2/` etc. Template context: `paginator.pages`, `paginator.next_url`, `paginator.prev_url`, `paginator.total_pages`
- [ ] **Search index** — generate a JSON index of all pages (`search-index.json`) for client-side search (e.g., Fuse.js). Include title, slug, summary, tags
- [ ] **404 page** — if `content/404.md` exists, render it as `public/404.html` (not `public/404/index.html`). Most static hosts serve this automatically
- [ ] **Redirects / aliases** — frontmatter `aliases: ["/old-url/"]` generates redirect HTML files at the old paths pointing to the new URL

## Robustness

- [ ] **Strip XML control characters in feeds** — the feed generator currently passes through invalid XML characters (`\x0B`, `\x00`, etc.). Strip characters outside the XML-valid range before writing feed output
- [ ] **Graceful template errors in `serve`** — template syntax errors during dev server rebuild currently print a stack trace and continue. Show the error in the browser via the WebSocket live-reload channel instead
- [ ] **Concurrent build safety** — audit all shared mutable state in the rayon parallel sections. The current code is safe (tested with Hugo regression test #3013) but has no formal proof or `loom` testing

## Performance (future)

- [ ] **Arena allocator for frontmatter** — replace per-page heap allocations with a bump allocator (`bumpalo`). Would require `Frontmatter<'arena>` and `Page<'arena>` lifetime parameters, rippling through the entire codebase. Expected impact: ~10-15ms on discovery (currently 147ms). Should be attempted on a feature branch due to the invasive type changes. See BENCHMARKS.md for design notes.
- [ ] **io_uring on Linux** — batch file create/write operations into kernel submission queues. Could reduce the output stage from 1,534ms to ~300-500ms on Linux. Platform-specific, requires `io-uring` crate and conditional compilation.
- [ ] **PGO (profile-guided optimization)** — two-pass compilation trained on the 10k benchmark workload. Expected 10-20% overall improvement. Requires CI integration for the profiling pass.
