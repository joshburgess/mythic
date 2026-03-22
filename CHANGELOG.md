# Changelog

All notable changes to Mythic will be documented in this file.

## [Unreleased]

### Added
- Full build pipeline: content discovery, frontmatter parsing (YAML/TOML), markdown rendering (pulldown-cmark with GFM), template application (Tera + Handlebars), file output with clean URLs
- Incremental builds with content-hash caching (`.mythic-cache.json`)
- Dev server with WebSocket live reload, CSS hot injection, and DOM reconciliation
- File watcher with 200ms debouncing for content, template, and config changes
- Asset pipeline: image optimization (WebP), CSS/JS bundling and minification, Sass/SCSS compilation (grass)
- Syntax highlighting via syntect with configurable themes and line numbers
- Shortcode system with self-closing and paired syntax, code block protection
- Data file loading from `_data/` (YAML, TOML, JSON) with nested namespaces
- Eleventy-style directory data cascade via `_dir.yaml` files
- Taxonomy system with tags, categories, and custom taxonomies
- Atom feed generation (site-wide and per-taxonomy)
- Sitemap.xml and robots.txt generation
- Table of contents extraction with anchor IDs and duplicate handling
- Internationalization: locale directories, hreflang tags, translation files
- Plugin system with Rust trait hooks and Rhai scripting
- Built-in ReadingTimePlugin
- Migration tools for Jekyll, Hugo, and Eleventy
- Link checker with internal link validation, alt text warnings, heading hierarchy checks
- Multi-engine templates: Tera (.html, .tera) and Handlebars (.hbs) side by side
- GitHub Action for build and deploy to GitHub Pages
- Four starter templates: blank, blog, docs, portfolio
- `mythic init --template <name>` scaffolding
- `mythic build --profile` per-stage timing breakdown
- `mythic check` content validation command
- `ugly_urls` flat output mode for faster builds
- Cross-platform release workflow (Linux x86_64/musl/aarch64, macOS x86_64/aarch64, Windows x86_64)
- Install script for binary downloads
- CI workflow with tests (Ubuntu/macOS/Windows), clippy, rustfmt
- Benchmark workflow with Hugo comparison
- 21-page documentation site built with Mythic itself
- `mythic new <type> "Title"` command for scaffolding content files with frontmatter
- `mythic --version` flag
- Colored CLI output (build summaries, errors, warnings)
- Config validation with warnings for unrecognized keys, empty base_url, trailing slashes
- Friendly template error messages with hints
- Pagination module with Paginator context for taxonomy term pages
- Search index generation (`search-index.json`) for client-side search
- 404 page handling (`content/404.md` renders as `public/404.html`)
- Redirect/alias support via frontmatter `aliases` field
- Build errors shown in browser during `mythic serve` via error overlay
- XML control character stripping in Atom feeds

### Performance
- Parallel markdown rendering via rayon
- Parallel template rendering via rayon
- Parallel file output with pre-created directory tree
- Incremental builds: 174ms for 10k unchanged pages (9.9x faster than Hugo)
- Flat URL mode: 1,338ms for 10k pages (22% faster than Hugo)
- Content hashing with ahash (fixed seeds for deterministic caching)
- CompactString for frontmatter fields (inline storage for strings <= 24 bytes)
- lasso string interning for deduplicated layout/tag values
- Thin LTO + codegen-units=1 release profile
- Pre-computed output paths to avoid redundant PathBuf joins

### Unique Differentiators (no other SSG has these)
- Content linting with configurable quality rules (word counts, required fields, orphan detection)
- Schema.org JSON-LD auto-generation from frontmatter (BlogPosting, Article, WebPage, BreadcrumbList)
- Build-time accessibility auditing (WCAG checks: alt text, lang, headings, links, viewport, zoom)
- Smart content diffing with deploy manifests for minimal deployments
- Rhai-powered computed frontmatter fields (`extra.field: "rhai: word_count / 200"`)

### Additional Features
- Content collections available in templates (`{{ data.pages }}`, `{{ data.sections.blog }}`)
- `mythic watch` command (rebuild without server)
- `mythic list` command (show all pages with dates and slugs)
- `mythic clean` command (delete output directory)
- `mythic build --json` structured output for CI
- `mythic completions <shell>` for bash/zsh/fish/powershell
- `--quiet` global flag for CI/scripting
- RSS 2.0 feed generation alongside Atom
- JSON Feed 1.1 generation
- Custom Tera filters: reading_time, word_count, truncate_words
- Content summaries via `<!--more-->` marker
- Markdown render hooks for customizing link/image output
- Obsidian-style admonitions (`> [!NOTE]`, `> [!WARNING]`, etc.)
- Math rendering (inline `$...$`, display `$$...$$`, KaTeX support)
- Related content engine (tag-based relevance scoring)
- Remote data fetching with filesystem caching and TTL
- Custom JSON output format per-page and site-wide API index
- SRI (Subresource Integrity) hashes for CSS/JS assets
- Built-in shortcode templates (youtube, figure, callout, details, github)
- Minimal starter template
- Starters embedded in binary via include_dir
- Draft preview hint in dev server
- Build errors shown in browser during `mythic serve`
- Duplicate slug detection with warnings
- Skip failing pages instead of aborting build
- `--version` flag

### Fixed
- Empty tags no longer produce empty-slug taxonomy terms
- "C++" and "C#" now slugify to distinct values (c-plus-plus, c-sharp)
- Shortcodes inside fenced code blocks are preserved as literal text
- All `unwrap()` calls removed from production code (4 crash risks eliminated)
- Sitemap URLs respect `ugly_urls` config
- XML-invalid control characters stripped from feed output
- Binary renamed from `mythic-cli` to `mythic`
