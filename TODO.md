# Mythic — Status

## Completed

### Core Build Pipeline
- [x] Content discovery, frontmatter parsing (YAML/TOML), markdown rendering, template application
- [x] Incremental builds with content-hash caching
- [x] Parallel rendering and file output
- [x] Clean URLs and flat URL mode (`ugly_urls`)
- [x] Draft filtering with `--drafts` flag

### Content Features
- [x] Syntax highlighting (syntect, configurable themes)
- [x] Shortcodes (self-closing + paired, code block protection)
- [x] Table of contents extraction
- [x] Content summaries (`<!--more-->` marker)
- [x] Admonitions (`> [!NOTE]`, `> [!WARNING]`, etc.)
- [x] Math rendering (inline/display/code blocks, KaTeX)
- [x] Markdown render hooks (customizable link/image output)

### Data & Templates
- [x] Data files (`_data/` YAML/TOML/JSON with nested namespaces)
- [x] Directory data cascade (`_dir.yaml`)
- [x] Content collections (`{{ data.pages }}`, `{{ data.sections }}`)
- [x] Multi-engine templates (Tera + Handlebars)
- [x] Custom Tera filters (reading_time, word_count, truncate_words)
- [x] Computed frontmatter (Rhai expressions)
- [x] Remote data fetching with caching

### Taxonomies & Feeds
- [x] Tags, categories, custom taxonomies with pagination
- [x] Atom, RSS 2.0, and JSON Feed 1.1 generation
- [x] Related content engine

### SEO & Quality
- [x] Sitemap.xml and robots.txt
- [x] Schema.org JSON-LD auto-generation
- [x] SRI integrity hashes for assets
- [x] Content linting (word counts, required fields, orphan detection)
- [x] Accessibility auditing (WCAG checks at build time)
- [x] Link checker with heading hierarchy validation
- [x] Smart content diffing with deploy manifests

### Other Features
- [x] Search index (JSON) for client-side search
- [x] Pagination
- [x] 404 page handling
- [x] Redirects/aliases
- [x] Custom output formats (JSON API)
- [x] i18n (locale dirs, hreflang, translations)
- [x] Plugin system (Rust hooks + Rhai scripting)
- [x] Migration tools (Jekyll, Hugo, Eleventy)

### CLI
- [x] `mythic init` with embedded starters (blank, blog, docs, portfolio, minimal)
- [x] `mythic new` content scaffolding
- [x] `mythic build` with --clean, --drafts, --profile, --quiet, --json
- [x] `mythic serve` with live reload, error overlay, draft hints
- [x] `mythic watch` rebuild-only mode
- [x] `mythic check` link/a11y validation
- [x] `mythic list` page listing
- [x] `mythic clean` output cleanup
- [x] `mythic migrate` from Jekyll/Hugo/Eleventy
- [x] `mythic completions` for bash/zsh/fish/powershell
- [x] `--version`, `--quiet` global flags
- [x] Colored output, config validation, friendly errors

### Quality
- [x] 433 tests (including regression tests from Hugo/Zola)
- [x] Zero clippy warnings
- [x] Zero production unwrap() calls
- [x] LICENSE, CHANGELOG, CONTRIBUTING, BENCHMARKS
- [x] CI workflows (test, clippy, fmt, build, docs, benchmark)
- [x] Release workflow (6 platform targets)
- [x] Cargo publish metadata

## Future (performance, feature branch)
- [ ] Arena allocator for frontmatter (bumpalo)
- [ ] io_uring on Linux
- [ ] Profile-guided optimization (PGO)
- [ ] Visual regression testing (requires headless browser)
