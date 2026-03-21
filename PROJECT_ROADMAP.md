# Mythic: Rust Static Site Generator — Claude Code Build Plan

> **How to use this document**: Each phase is broken into discrete iterations. Feed each iteration to Claude Code as a self-contained task. Every iteration ends with concrete acceptance criteria so the agent knows when it's done. Complete iterations in order — each builds on the last.

---

## Phase 0: Project Scaffolding & Workspace Setup

### Iteration 0.1 — Initialize the Rust workspace

**Prompt for Claude Code:**
> Create a new Rust workspace called `mythic` with the following crate structure. Use Cargo workspaces. Each crate should compile and have a passing `#[test]` placeholder. Initialize a git repo with a `.gitignore` for Rust.

```
mythic/
├── Cargo.toml              # workspace root
├── crates/
│   ├── mythic-core/        # config, VFS, pipeline orchestration
│   ├── mythic-markdown/    # frontmatter parsing, markdown rendering
│   ├── mythic-template/    # multi-engine template system
│   ├── mythic-assets/      # image/CSS/JS optimization
│   ├── mythic-server/      # dev server with HMR
│   └── mythic-cli/         # CLI binary (clap)
├── fixtures/               # test sites for integration tests
│   └── basic-site/
│       ├── content/
│       │   └── hello.md
│       ├── templates/
│       │   └── default.html
│       └── mythic.toml
├── .gitignore
└── README.md
```

**Acceptance criteria:**
- `cargo build --workspace` succeeds with no errors
- `cargo test --workspace` passes (placeholder tests)
- Each crate has a `lib.rs` (or `main.rs` for `mythic-cli`) with a doc comment describing its purpose
- `fixtures/basic-site/` contains a minimal valid test site with one markdown file, one template, and a config file
- `mythic.toml` config file has fields: `title`, `base_url`, `content_dir`, `output_dir`, `template_dir`

---

### Iteration 0.2 — Core config and error handling

**Prompt for Claude Code:**
> In `mythic-core`, implement the `SiteConfig` struct with serde deserialization from TOML. Support these fields: `title` (String), `base_url` (String), `content_dir` (PathBuf, default `"content"`), `output_dir` (PathBuf, default `"public"`), `template_dir` (PathBuf, default `"templates"`), `data_dir` (PathBuf, default `"_data"`). Use `anyhow` for error handling throughout the workspace. Add a `mythic_core::config::load_config(path: &Path) -> Result<SiteConfig>` function. Write tests that load the fixture config.

**Acceptance criteria:**
- `load_config` parses `fixtures/basic-site/mythic.toml` correctly
- Missing optional fields get defaults
- Invalid TOML returns a clear error message
- At least 3 unit tests covering: valid config, missing optional fields, invalid file

---

## Phase 1: Minimum Viable Build Pipeline

> Goal: `mythic build` takes markdown files → applies templates → outputs HTML. No frills.

### Iteration 1.1 — Frontmatter parsing

**Prompt for Claude Code:**
> In `mythic-markdown`, implement frontmatter parsing. Support both YAML (`---` delimiters) and TOML (`+++` delimiters). Create a `Frontmatter` struct with: `title` (String), `date` (Option<String>), `draft` (Option<bool>), `layout` (Option<String>, default `"default"`), `tags` (Option<Vec<String>>), `extra` (Option<HashMap<String, serde_json::Value>>). Implement `parse_frontmatter(raw: &str) -> Result<(Frontmatter, String)>` that returns the parsed frontmatter and the remaining body content. Write thorough tests.

**Acceptance criteria:**
- Parses YAML frontmatter between `---` delimiters
- Parses TOML frontmatter between `+++` delimiters
- Returns the body content (everything after the closing delimiter) with leading whitespace trimmed
- Errors clearly on unclosed delimiters
- At least 5 unit tests: YAML happy path, TOML happy path, missing optional fields, unclosed delimiter, no frontmatter at all (error)

---

### Iteration 1.2 — Markdown rendering

**Prompt for Claude Code:**
> In `mythic-markdown`, implement markdown-to-HTML rendering using `pulldown-cmark`. Enable these extensions: tables, footnotes, strikethrough, task lists. Create a `Page` struct in `mythic-core` that holds: `source_path`, `slug`, `frontmatter`, `raw_content`, `rendered_html` (Option), `output_path` (Option), `content_hash` (u64). Implement `render_markdown(pages: &mut [Page])` that uses `rayon` to render all pages in parallel. Write tests using the fixture site.

**Acceptance criteria:**
- Markdown renders to correct HTML (test with a fixture `.md` containing headers, lists, code blocks, links, bold/italic, a table, and a task list)
- Rendering is parallelized with rayon
- `content_hash` is computed from the raw file contents using `DefaultHasher`
- At least 3 tests: basic markdown, GFM extensions, empty body

---

### Iteration 1.3 — Content discovery

**Prompt for Claude Code:**
> In `mythic-core`, implement `discover_content(config: &SiteConfig) -> Result<Vec<Page>>`. It should recursively walk `config.content_dir`, find all `.md` and `.markdown` files, parse their frontmatter and body, compute content hashes, and return a Vec of `Page` structs. Ignore files starting with `_` or `.`. Derive the slug from the file path relative to content_dir (e.g., `content/blog/my-post.md` → slug `blog/my-post`). Write tests using the fixture site.

**Acceptance criteria:**
- Discovers all markdown files recursively
- Skips hidden files and files starting with `_`
- Slugs are derived from relative paths with the extension stripped
- Works with nested directories
- At least 3 tests: flat directory, nested directories, hidden/underscore files skipped

---

### Iteration 1.4 — Template engine integration

**Prompt for Claude Code:**
> In `mythic-template`, integrate the `tera` template engine. Implement `TemplateEngine::new(template_dir: &Path) -> Result<Self>` that loads all `.html` files from the template directory. Implement `TemplateEngine::render(page: &Page, site_config: &SiteConfig) -> Result<String>` that renders a page using the layout specified in its frontmatter (defaulting to `default`). The template context should include: `page` (frontmatter fields), `content` (the rendered HTML), `site` (title, base_url). Update the fixture site's `default.html` template to be a valid HTML5 document that renders `{{ content | safe }}` inside a body tag. Write tests.

**Acceptance criteria:**
- Templates are loaded from the configured directory
- `{{ content | safe }}` renders the page's HTML without escaping
- `{{ page.title }}`, `{{ site.title }}`, `{{ site.base_url }}` all resolve correctly
- Missing layout template returns a clear error
- At least 3 tests: default layout, custom layout name, missing template error

---

### Iteration 1.5 — Build orchestrator and file output

**Prompt for Claude Code:**
> In `mythic-core`, implement `build(config: &SiteConfig) -> Result<BuildReport>` that runs the full pipeline: discover → render markdown → apply templates → write output files. The `BuildReport` struct should contain: `total_pages`, `pages_written`, `pages_skipped` (drafts), `elapsed_ms`. Output paths should follow clean URL convention: slug `blog/my-post` → `public/blog/my-post/index.html`. Skip pages where `draft: true`. Print a summary line to stdout. Write an integration test that builds the fixture site and verifies the output.

**Acceptance criteria:**
- `build()` produces correct HTML files in the output directory
- Clean URLs: each page gets its own directory with `index.html`
- Drafts are skipped
- Output directory is created if it doesn't exist
- `BuildReport` has accurate counts and timing
- Integration test: build fixture site, read output file, verify it contains expected HTML content

---

### Iteration 1.6 — CLI binary

**Prompt for Claude Code:**
> In `mythic-cli`, implement the CLI using `clap` (derive API). Support these commands:
> - `mythic build` — runs the build pipeline. Flags: `--config <path>` (default `mythic.toml`), `--drafts` (include drafts), `--clean` (delete output dir first)
> - `mythic init <name>` — creates a new project directory with a skeleton: `mythic.toml`, `content/index.md`, `templates/default.html`, and a `.gitignore`
>
> The build command should load config, run `mythic_core::build()`, and print the report. The init command should create a working starter site that builds successfully. Write integration tests that run the binary as a subprocess.

**Acceptance criteria:**
- `cargo run -- init test-site` creates a valid project that `cargo run -- build --config test-site/mythic.toml` builds successfully
- `--drafts` flag includes draft pages
- `--clean` deletes the output directory before building
- `--config` allows a custom config path
- `mythic build` with no args looks for `mythic.toml` in the current directory
- Help text is clear and useful (`--help`)
- At least 2 integration tests: init + build round trip, build with --drafts

---

## Phase 2: Incremental Builds & Dev Server

> Goal: fast feedback loop — watch files, rebuild only what changed, live-reload in browser.

### Iteration 2.1 — Dependency graph and incremental cache

**Prompt for Claude Code:**
> In `mythic-core`, implement an incremental build system. Create a `DepGraph` struct that tracks content hashes from the previous build, persisted to `.mythic-cache.json` in the output directory. Before writing each page, check if its `content_hash` matches the cached hash — if so, skip it. After the build, save the updated cache. Update `build()` to use the dep graph, and update `BuildReport` to include `pages_unchanged` count. Write tests proving that a second build with no changes writes zero files.

**Acceptance criteria:**
- First build writes all pages and creates the cache file
- Second identical build writes 0 pages (all unchanged)
- Changing one file rebuilds only that file
- Deleting the cache forces a full rebuild
- `BuildReport` accurately reports `pages_written` vs `pages_unchanged`
- At least 3 tests: full build, no-op rebuild, single file changed

---

### Iteration 2.2 — File watcher

**Prompt for Claude Code:**
> In `mythic-server`, implement a file watcher using the `notify` crate. Create `FileWatcher::new(config: &SiteConfig) -> Result<Self>` that watches the content, template, and data directories. It should debounce events (200ms) and return a channel of `WatchEvent` enums: `ContentChanged(PathBuf)`, `TemplateChanged(PathBuf)`, `ConfigChanged`. On content change, trigger an incremental rebuild. On template change, trigger a full rebuild (templates can affect any page). On config change, reload config and full rebuild. Write tests using temp directories.

**Acceptance criteria:**
- Watches content, template, and data directories recursively
- Events are debounced (rapid saves don't trigger multiple rebuilds)
- Different event types trigger appropriate rebuild strategies
- Watcher runs in a background thread and sends events via `crossbeam-channel` or `std::sync::mpsc`
- At least 2 tests: file change detected, debouncing works

---

### Iteration 2.3 — Dev server with live reload

**Prompt for Claude Code:**
> In `mythic-server`, implement a development HTTP server using `axum`. It should serve static files from the output directory, inject a live-reload `<script>` tag into HTML responses before `</body>`, and expose a WebSocket endpoint at `/__mythic/ws`. When the file watcher detects a change and the rebuild completes, send a `reload` message over the WebSocket. The client script should listen for this and call `location.reload()`. Implement `serve(config: &SiteConfig, port: u16) -> Result<()>`. Add `mythic serve` to the CLI with `--port` (default 3000) and `--open` (opens browser) flags.

**Acceptance criteria:**
- `mythic serve` starts a server, builds the site, and watches for changes
- HTML files served have the live-reload script injected
- Changing a content file triggers rebuild and browser reload
- Non-HTML files (CSS, JS, images) are served correctly without script injection
- `--port` flag works
- Server prints the local URL on startup
- Graceful shutdown on Ctrl+C

---

### Iteration 2.4 — Hot CSS/HTML injection (upgrade from full reload)

**Prompt for Claude Code:**
> Upgrade the live-reload system to support hot injection. When only CSS files change, send a `css-reload` WebSocket message with the changed file path — the client script should update the stylesheet's `href` with a cache-busting query param instead of doing a full page reload. When a content-only change occurs (no layout change), send an `html-update` message with the new `<main>` or `<article>` content — the client script should replace the element's innerHTML via `morphdom` (include a vendored minimal DOM-diffing function, ~50 lines). Fall back to full reload for template/config changes.

**Acceptance criteria:**
- CSS-only changes update styles without full page reload
- Content-only changes update the page body without full reload
- Template changes still trigger full reload
- The injected client script is <5KB minified
- Falls back gracefully if WebSocket disconnects (retry with backoff)

---

## Phase 3: Asset Pipeline

> Goal: built-in image optimization, CSS processing, JS bundling, content hashing.

### Iteration 3.1 — Image processing

**Prompt for Claude Code:**
> In `mythic-assets`, implement automatic image optimization. Scan the content and a configured `static/` directory for images (jpg, png, gif, webp). For each image, generate: a WebP version, resized variants at configurable breakpoints (default: 400, 800, 1200 pixels wide), and keep the original. Output to `public/assets/img/` with content-hashed filenames (e.g., `photo-a1b2c3d4-800.webp`). Use the `image` crate. Implement a `process_images(config: &SiteConfig) -> Result<ImageManifest>` that returns a manifest mapping original paths to their generated variants. Parallelize with rayon. Write tests with a fixture image.

**Acceptance criteria:**
- JPEG and PNG images are converted to WebP
- Multiple sizes are generated per configurable breakpoints
- Filenames include a content hash for cache busting
- Original images are also copied to output (for fallback)
- `ImageManifest` maps `original_path → Vec<GeneratedImage { path, width, format }>`
- Processing is parallelized
- Skips images that haven't changed (hash check)
- At least 3 tests: JPEG processing, PNG processing, skip unchanged

---

### Iteration 3.2 — Responsive image shortcode

**Prompt for Claude Code:**
> Create a Tera template function (custom function) called `image` that generates responsive `<picture>` elements. Usage in templates: `{{ image(src="photo.jpg", alt="A photo", sizes="(max-width: 800px) 100vw, 800px") }}`. It should look up the image in the `ImageManifest` and output a `<picture>` tag with `<source>` elements for WebP at each breakpoint and an `<img>` fallback. Also implement a markdown shortcode syntax: `{% image "photo.jpg" "alt text" %}` that gets preprocessed before markdown rendering. Register the function with the template engine.

**Acceptance criteria:**
- `{{ image(src="photo.jpg", alt="text") }}` outputs a valid `<picture>` element with WebP sources and an `<img>` fallback
- `sizes` attribute is configurable, with a sensible default
- `loading="lazy"` is included by default
- Width and height attributes are set on the `<img>` to prevent layout shift
- Markdown shortcode syntax works in content files
- Error if referenced image doesn't exist in the manifest
- At least 3 tests: basic output, custom sizes, missing image error

---

### Iteration 3.3 — CSS and JS processing

**Prompt for Claude Code:**
> In `mythic-assets`, implement CSS and JS processing. For CSS: concatenate all CSS files from a configured `styles/` directory (in alphabetical order, or via a manifest), minify using a lightweight approach (strip comments, collapse whitespace — use the `css-minify` crate or implement basic minification), and output a single hashed file (`styles-{hash}.css`). For JS: concatenate all JS files from a `scripts/` directory, minify (strip comments, collapse whitespace), and output a single hashed file (`scripts-{hash}.js`). Make the hashed filenames available to templates via `{{ assets.css }}` and `{{ assets.js }}`. Add these to the build pipeline.

**Acceptance criteria:**
- CSS files are concatenated and minified into a single output file
- JS files are concatenated and minified into a single output file
- Output filenames include content hashes
- Templates can reference the output files via `{{ assets.css_path }}` and `{{ assets.js_path }}`
- Files are only regenerated when inputs change
- At least 3 tests: CSS concatenation + minification, JS concatenation + minification, hash changes when content changes

---

### Iteration 3.4 — Sass/SCSS support

**Prompt for Claude Code:**
> Add Sass/SCSS compilation support to the CSS pipeline. Use the `grass` crate (pure Rust Sass compiler). If the styles directory contains `.scss` or `.sass` files, compile them before the concatenation/minification step. Support `@import` and `@use` between Sass files. The output should still be a single minified, hashed CSS file. Add a config option `sass: { enabled: true, style: "compressed" }` to `mythic.toml`.

**Acceptance criteria:**
- `.scss` files in the styles directory are compiled to CSS
- `@import` between Sass files resolves correctly
- Compiled Sass is included in the concatenated/minified output
- Plain `.css` files and `.scss` files can coexist
- Config option to enable/disable Sass
- At least 3 tests: basic SCSS compilation, imports, mixed CSS + SCSS

---

## Phase 4: Data System & Taxonomies

> Goal: data files, data cascade, taxonomies (tags, categories), feeds.

### Iteration 4.1 — Data file loading

**Prompt for Claude Code:**
> In `mythic-core`, implement a data loading system. Load all YAML, TOML, and JSON files from `config.data_dir` (default `_data/`) and make them available in templates under `{{ data.<filename> }}`. For example, `_data/authors.yaml` becomes `{{ data.authors }}`. Support nested directories: `_data/nav/main.yaml` → `{{ data.nav.main }}`. Load data files once at build start and pass them into the template context. Write tests.

**Acceptance criteria:**
- YAML, TOML, and JSON data files are loaded
- Data is accessible in templates under the `data` namespace
- Nested directories create nested namespaces
- Invalid data files produce clear error messages with the file path
- At least 3 tests: YAML loading, nested directory namespace, invalid file error

---

### Iteration 4.2 — Directory-level data cascade

**Prompt for Claude Code:**
> Implement an Eleventy-style data cascade. If a directory contains a `_dir.yaml` (or `.toml`/`.json`) file, its fields are merged into the frontmatter of all pages in that directory and its subdirectories. Child `_dir.yaml` overrides parent. Page-level frontmatter overrides directory-level data. Implement this as a preprocessing step in the build pipeline between discovery and markdown rendering. The merge order (lowest to highest priority) is: root `_dir.yaml` → nested `_dir.yaml` → page frontmatter.

**Acceptance criteria:**
- `_dir.yaml` in a directory applies to all pages in that directory
- Nested `_dir.yaml` overrides parent values
- Page frontmatter overrides directory data
- Deep merge on maps (not replace)
- Works with YAML, TOML, and JSON
- At least 4 tests: basic cascade, nested override, page override, deep merge behavior

---

### Iteration 4.3 — Taxonomies (tags, categories)

**Prompt for Claude Code:**
> Implement a taxonomy system. In `mythic.toml`, allow defining taxonomies:
> ```toml
> [[taxonomies]]
> name = "tags"
> slug = "tags"
> feed = true
>
> [[taxonomies]]
> name = "categories"
> slug = "category"
> feed = false
> ```
> For each taxonomy, generate: a listing page at `/{slug}/` showing all terms, and a term page at `/{slug}/{term}/` listing all pages with that term. Use templates `taxonomy_list.html` and `taxonomy_term.html`. The template context should include the term name and the list of pages. Write tests.

**Acceptance criteria:**
- Taxonomy terms are extracted from page frontmatter
- Listing pages are generated for each taxonomy
- Term pages are generated for each unique term value
- Templates receive the correct context (`term.name`, `term.pages`)
- Pages are sorted by date (newest first) on term pages
- At least 3 tests: tag listing generated, tag term page generated, multiple taxonomies work independently

---

### Iteration 4.4 — RSS/Atom feed generation

**Prompt for Claude Code:**
> Implement Atom feed generation. Generate a site-wide feed at `/feed.xml` and per-taxonomy feeds at `/{taxonomy_slug}/{term}/feed.xml` (if the taxonomy has `feed = true`). Use a Tera template for the feed XML rather than a separate library. Include: title, link, updated date, author (from config), and the most recent 20 entries with title, link, published date, and a summary (first 200 chars of content, HTML-stripped). Add config: `[feed] title = "..." author = "..." entries = 20`.

**Acceptance criteria:**
- Site-wide `feed.xml` is valid Atom XML
- Per-taxonomy feeds are generated when `feed = true`
- Feed entries include title, link, date, and summary
- Only the most recent N entries are included (configurable)
- Feed validates against an Atom validator
- At least 3 tests: site feed generated, taxonomy feed generated, entry limit respected

---

## Phase 5: Shortcodes, Syntax Highlighting & Advanced Markdown

### Iteration 5.1 — Syntax highlighting

**Prompt for Claude Code:**
> Integrate `syntect` for syntax highlighting in code blocks. When rendering markdown, detect fenced code blocks with language annotations (e.g., ` ```rust `) and apply syntax highlighting using syntect's HTML output with CSS classes. Generate a syntax highlight CSS file as part of the asset pipeline (supporting multiple themes). Add config: `[highlight] theme = "base16-ocean.dark" line_numbers = false`. Use syntect's built-in theme set. Write tests.

**Acceptance criteria:**
- Fenced code blocks with language tags get syntax highlighting
- HTML output uses CSS classes (not inline styles) for theming
- A syntax CSS file is generated and included in the asset pipeline
- `theme` config option switches themes
- `line_numbers` option adds line number spans
- Code blocks without a language tag are rendered as plain `<pre><code>`
- At least 3 tests: highlighted Rust code, plain code block, line numbers

---

### Iteration 5.2 — Custom shortcodes

**Prompt for Claude Code:**
> Implement a shortcode system for markdown content. Shortcodes are invoked with `{{% name arg1="val" arg2="val" %}}` syntax (paired: `{{% name %}}content{{% /name %}}` or self-closing). Shortcodes are defined as Tera templates in a `shortcodes/` directory (e.g., `shortcodes/youtube.html`). Implement a preprocessing step that runs before markdown rendering: scan the raw content for shortcode patterns, render each shortcode template with the provided arguments (and inner content for paired shortcodes), and replace the shortcode invocation with the rendered HTML. Write tests.

**Acceptance criteria:**
- Self-closing shortcodes: `{{% youtube id="abc123" %}}` renders `shortcodes/youtube.html` with `id` in context
- Paired shortcodes: `{{% callout type="warning" %}}text{{% /callout %}}` renders with `inner` content available
- Shortcode templates are Tera templates with full access to shortcode args
- Nested shortcodes work (at least one level deep)
- Missing shortcode template produces a clear error
- At least 4 tests: self-closing, paired, nested, missing template error

---

### Iteration 5.3 — Table of contents generation

**Prompt for Claude Code:**
> Implement automatic table of contents generation. During markdown rendering, extract all headings (h1-h6) with their text, level, and auto-generated anchor IDs. Make the TOC available in templates as `{{ page.toc }}` — an array of `{ level, text, id }` objects. Also provide a `{{ toc() }}` Tera function that renders a nested `<nav>` with `<ul>`/`<li>` elements. Add `id` attributes to rendered heading elements so anchor links work. Add config: `[toc] min_level = 2 max_level = 4`.

**Acceptance criteria:**
- Headings get `id` attributes based on their text (slugified, e.g., "My Heading" → `my-heading`)
- Duplicate heading IDs get suffixed (`my-heading-1`, `my-heading-2`)
- `page.toc` is available in templates as structured data
- `{{ toc() }}` renders a nested HTML navigation
- `min_level` and `max_level` config options filter which headings appear
- At least 3 tests: basic TOC extraction, duplicate ID handling, nested list structure

---

## Phase 6: Plugin System

### Iteration 6.1 — Hook-based plugin architecture

**Prompt for Claude Code:**
> Design and implement a plugin hook system in `mythic-core`. Define these hook points: `on_pre_build`, `on_page_discovered(page)`, `on_pre_render(page)`, `on_post_render(page)`, `on_post_build(site)`. Create a `Plugin` trait:
> ```rust
> pub trait Plugin: Send + Sync {
>     fn name(&self) -> &str;
>     fn on_pre_build(&self, config: &SiteConfig) -> Result<()> { Ok(()) }
>     fn on_page_discovered(&self, page: &mut Page) -> Result<()> { Ok(()) }
>     fn on_pre_render(&self, page: &mut Page) -> Result<()> { Ok(()) }
>     fn on_post_render(&self, page: &mut Page) -> Result<()> { Ok(()) }
>     fn on_post_build(&self, report: &BuildReport) -> Result<()> { Ok(()) }
> }
> ```
> Create a `PluginManager` that stores `Vec<Box<dyn Plugin>>` and calls hooks at the appropriate pipeline stages. Implement one built-in plugin as a proof of concept: `ReadingTimePlugin` that calculates reading time and adds it to `page.extra["reading_time"]`. Write tests.

**Acceptance criteria:**
- `Plugin` trait is defined with default no-op implementations for all hooks
- `PluginManager` executes hooks in registration order
- Hooks can mutate pages (for `on_page_discovered`, `on_pre_render`, `on_post_render`)
- `ReadingTimePlugin` correctly estimates reading time (~200 words per minute)
- Plugins that return errors halt the build with a clear message
- At least 4 tests: hook execution order, page mutation, error propagation, reading time calculation

---

### Iteration 6.2 — Rhai scripting for user plugins

**Prompt for Claude Code:**
> Integrate the `rhai` scripting engine to allow users to write plugins in a simple scripting language. Users place `.rhai` files in a `plugins/` directory. Each script can register functions for hooks. Implement a `RhaiPluginLoader` that scans the plugins directory, loads each script, and wraps it as a `Plugin` impl. Expose to Rhai: page frontmatter (read/write), page content, config values. Provide a Rhai example plugin: `word-count.rhai` that adds a `word_count` field to each page's extra data. Write tests.

**Acceptance criteria:**
- `.rhai` files in `plugins/` are automatically loaded
- Rhai scripts can read and modify page frontmatter and extra data
- Rhai scripts can access site config values
- Errors in Rhai scripts produce clear messages with the script filename and line number
- Example `word-count.rhai` plugin works correctly
- At least 3 tests: Rhai plugin loads and executes, modifies page data, script error handling

---

## Phase 7: Migration Tools

### Iteration 7.1 — Jekyll migration

**Prompt for Claude Code:**
> Implement `mythic migrate --from jekyll --source <path> --output <path>`. It should:
> 1. Read `_config.yml` and convert it to `mythic.toml`
> 2. Copy `_posts/` to `content/posts/`, converting Jekyll filename format (`YYYY-MM-DD-title.md`) to Mythic format (extract date into frontmatter, use just the title slug as filename)
> 3. Copy `_layouts/` to `templates/`, converting Liquid template syntax to Tera syntax (map the most common patterns: `{{ content }}` → `{{ content | safe }}`, `{% for %}` loops, `{% if %}` conditionals, `{{ page.* }}` variables, `{% include %}` → `{% include %}`)
> 4. Copy `_includes/` to `templates/partials/`
> 5. Copy `_data/` to `_data/`
> 6. Copy static assets
> 7. Print a report of what was converted and what needs manual attention

**Acceptance criteria:**
- Jekyll `_config.yml` → valid `mythic.toml` with mapped fields
- Posts are renamed and dates moved to frontmatter
- Most common Liquid → Tera conversions work (at least: content rendering, for loops, if/else, includes, variable access)
- A report lists any Liquid syntax that couldn't be auto-converted
- The migrated site builds successfully with `mythic build` (even if some templates need manual fixes)
- At least 4 tests: config conversion, post migration, template conversion, full round trip

---

### Iteration 7.2 — Hugo migration

**Prompt for Claude Code:**
> Implement `mythic migrate --from hugo --source <path> --output <path>`. It should:
> 1. Read `config.toml` (or `hugo.toml`/`config.yaml`) and convert to `mythic.toml`
> 2. Copy `content/` preserving directory structure, converting Hugo-specific frontmatter fields (e.g., `type`, `weight`) to `extra`
> 3. Convert Go template syntax in `layouts/` to Tera syntax (map: `{{ .Title }}` → `{{ page.title }}`, `{{ .Content }}` → `{{ content | safe }}`, `range` → `for`, `partial` → `include`, `.Params.*` → `page.extra.*`)
> 4. Copy `static/` to `static/`
> 5. Copy `data/` to `_data/`
> 6. Convert Hugo shortcodes in `layouts/shortcodes/` to Mythic shortcode templates
> 7. Print a conversion report

**Acceptance criteria:**
- Hugo config formats (TOML, YAML) → valid `mythic.toml`
- Content files are copied with frontmatter adjustments
- Most common Go template → Tera conversions work
- Hugo shortcodes are converted to Mythic shortcode format
- Conversion report lists unconverted patterns
- At least 4 tests: config, content, template conversion, shortcode conversion

---

### Iteration 7.3 — Eleventy migration

**Prompt for Claude Code:**
> Implement `mythic migrate --from eleventy --source <path> --output <path>`. It should:
> 1. Read `.eleventy.js` or `eleventy.config.js` and extract input/output dirs, template formats, and data config → convert to `mythic.toml`
> 2. Copy content files (markdown, Nunjucks, Liquid) to `content/`, converting Nunjucks templates to Tera (closest syntax match)
> 3. Copy `_includes/` to `templates/`
> 4. Copy `_data/` to `_data/` (JS data files get a warning — they need manual conversion)
> 5. Handle Eleventy's directory data files (`dirname.json`) → convert to `_dir.yaml`
> 6. Print a conversion report

**Acceptance criteria:**
- Eleventy JS config is parsed for basic settings (input/output dirs, data dir)
- Nunjucks → Tera template conversion for common patterns
- Directory data files are converted
- JS data files are flagged for manual conversion (can't auto-convert arbitrary JS)
- Conversion report is accurate
- At least 3 tests: config extraction, Nunjucks conversion, directory data conversion

---

## Phase 8: SEO, Accessibility & Quality Tools

### Iteration 8.1 — Sitemap and robots.txt generation

**Prompt for Claude Code:**
> Implement automatic `sitemap.xml` and `robots.txt` generation. The sitemap should include all non-draft pages with `<loc>`, `<lastmod>` (from file mtime or frontmatter date), and `<changefreq>` (configurable per-directory via `_dir.yaml`). Generate `robots.txt` that points to the sitemap. Add config: `[sitemap] enabled = true changefreq = "weekly"`. Pages can opt out with `sitemap: false` in frontmatter. Write tests.

**Acceptance criteria:**
- Valid `sitemap.xml` is generated in the output root
- All non-draft, non-excluded pages are included
- URLs use the configured `base_url`
- `robots.txt` references the sitemap
- Pages can opt out via frontmatter
- At least 3 tests: sitemap content, page exclusion, robots.txt content

---

### Iteration 8.2 — Link checker and content validator

**Prompt for Claude Code:**
> Implement `mythic check` CLI command. It should:
> 1. **Internal link checking**: Parse all rendered HTML, extract all `<a href>` and `<img src>` references, verify that internal links resolve to existing output files
> 2. **External link checking**: Optionally (`--external`) send HEAD requests to external URLs and report broken links (with configurable timeout and retry)
> 3. **Image alt text**: Warn on `<img>` tags missing `alt` attributes
> 4. **Heading hierarchy**: Warn if heading levels skip (e.g., h1 → h3 with no h2)
>
> Output results as a structured report: errors (broken links), warnings (missing alt text, heading skips), and a summary. Use `reqwest` for external checks with `tokio` runtime. Write tests.

**Acceptance criteria:**
- Internal broken links are detected and reported with the source file and line
- External link checking works with concurrent requests (configurable concurrency)
- Missing alt text produces warnings
- Heading hierarchy violations produce warnings
- Exit code is non-zero if any errors are found (warnings don't affect exit code)
- At least 4 tests: broken internal link detected, valid internal link passes, missing alt text warning, heading skip warning

---

## Phase 9: Multi-Template Engine & i18n

### Iteration 9.1 — Multi-engine template support

**Prompt for Claude Code:**
> Upgrade `mythic-template` to support multiple template engines. Detect the engine from file extension: `.html` or `.tera` → Tera, `.hbs` → Handlebars. Use the `handlebars` crate for Handlebars support. Create a `TemplateRenderer` trait that both engines implement, and a dispatcher that picks the right engine per template. Users should be able to mix engines in the same project (e.g., some layouts in Tera, some in Handlebars). Add a config option `[templates] default_engine = "tera"` for ambiguous cases. Write tests for both engines.

**Acceptance criteria:**
- Tera templates (`.tera`, `.html`) render correctly
- Handlebars templates (`.hbs`) render correctly
- A single project can use both engines
- The same template context (page, site, data) is available in both engines
- Config option sets the default for `.html` files
- At least 4 tests: Tera rendering, Handlebars rendering, mixed project, fallback to default engine

---

### Iteration 9.2 — Internationalization (i18n)

**Prompt for Claude Code:**
> Implement first-class i18n support. Config:
> ```toml
> [i18n]
> default_locale = "en"
> locales = ["en", "es", "fr"]
> ```
> Content structure: `content/en/about.md`, `content/es/about.md`, etc. Or alternatively, a single file with `locale: es` in frontmatter. Generate locale-prefixed URLs: `/es/about/`. Generate `<link rel="alternate" hreflang="es">` tags automatically. Provide `{{ t(key) }}` template function that looks up translations from `_data/i18n/{locale}.yaml`. Add `{{ page.translations }}` containing links to other language versions. Write tests.

**Acceptance criteria:**
- Content in locale directories generates locale-prefixed URLs
- `hreflang` link tags are automatically injected
- `{{ t("key") }}` looks up translations from locale data files
- `{{ page.translations }}` lists available translations with their URLs
- Default locale can optionally omit the prefix (`/about/` instead of `/en/about/`)
- At least 4 tests: locale URL generation, hreflang tags, translation lookup, translations list

---

## Phase 10: Performance, Benchmarks & Polish

### Iteration 10.1 — Benchmark suite

**Prompt for Claude Code:**
> Create a benchmark suite in `benches/` using `criterion`. Generate synthetic test sites of various sizes: 100, 1000, 10000, and 50000 pages. Each page should have realistic frontmatter, ~500 words of lorem ipsum markdown with headings, lists, code blocks, and links. Benchmark: full build time, incremental rebuild (1 file changed), markdown rendering only, template rendering only. Also create a `scripts/benchmark-comparison.sh` that builds the same content with Jekyll, Hugo, and Eleventy (if installed) and prints a comparison table. Write a `BENCHMARKS.md` with instructions.

**Acceptance criteria:**
- Criterion benchmarks run with `cargo bench`
- Synthetic site generator creates realistic content at configurable scales
- Benchmarks cover: full build, incremental build, markdown stage, template stage
- Comparison script produces a readable table of times
- `BENCHMARKS.md` documents how to run benchmarks and reproduce results
- Benchmarks are deterministic (seeded random content)

---

### Iteration 10.2 — Build profiling and optimization

**Prompt for Claude Code:**
> Add a `--profile` flag to `mythic build` that prints a detailed timing breakdown of each pipeline stage: discovery, frontmatter parsing, markdown rendering, template rendering, asset processing, file output. Identify and optimize bottlenecks. Specific optimizations to implement:
> 1. Use `memmap2` for reading large files instead of `read_to_string`
> 2. Use a string interner (`lasso` crate) for frequently repeated strings (layout names, tag values, etc.)
> 3. Pre-allocate output strings based on input size estimates
> 4. Parallelize file I/O writes with `rayon`
>
> After optimization, run the benchmark suite and record the improvement.

**Acceptance criteria:**
- `--profile` flag prints per-stage timing in a clear table format
- Memory-mapped file reading is used for files above a size threshold (e.g., 64KB)
- String interning is used for tag values and layout names
- File writes are parallelized
- Benchmark results show measurable improvement (document before/after in a commit message)

---

### Iteration 10.3 — Single binary distribution

**Prompt for Claude Code:**
> Set up CI/CD for cross-platform binary releases. Create a GitHub Actions workflow (`.github/workflows/release.yml`) that on tag push:
> 1. Builds release binaries for: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` (static), `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
> 2. Creates a GitHub Release with all binaries attached
> 3. Generates a SHA256 checksums file
> 4. Updates a Homebrew formula in a `homebrew-tap` repo
>
> Also create an install script: `curl -fsSL https://raw.githubusercontent.com/OWNER/mythic/main/install.sh | sh`

**Acceptance criteria:**
- GitHub Actions workflow builds for all 6 targets
- Release artifacts are uploaded to GitHub Releases
- Checksums file is included
- Homebrew formula is generated (can be a template that gets filled in)
- Install script detects OS/arch and downloads the right binary
- Workflow triggers on `v*` tag pushes

---

## Phase 11: GitHub Pages Integration

### Iteration 11.1 — GitHub Pages Action

**Prompt for Claude Code:**
> Create a GitHub Action in `action/` directory. The action should:
> 1. Download the correct Mythic binary for the runner OS
> 2. Cache the binary between runs
> 3. Run `mythic build` with configurable options
> 4. Deploy to GitHub Pages using `actions/upload-pages-artifact` and `actions/deploy-pages`
>
> Create `action.yml` with inputs: `version` (default `latest`), `config` (default `mythic.toml`), `build_args` (extra build flags). Publish example workflow file users can copy. Write a comprehensive README for the action.

```yaml
# Example usage users would add to their repo:
name: Deploy to GitHub Pages
on:
  push:
    branches: [main]
permissions:
  contents: read
  pages: write
  id-token: write
jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: owner/mythic-action@v1
        with:
          version: latest
```

**Acceptance criteria:**
- `action.yml` is valid and defines all inputs with descriptions
- Binary is downloaded and cached
- Build runs and output is deployed to Pages
- Example workflow file is provided and documented
- README covers: setup, configuration options, custom domains, troubleshooting
- Works with GitHub Pages' required permissions model

---

### Iteration 11.2 — Starter templates and themes

**Prompt for Claude Code:**
> Create 4 starter templates, each in its own directory under `starters/`:
> 1. `blog` — a clean, responsive blog with post listing, tag pages, RSS feed, syntax highlighting. Minimal CSS, no framework.
> 2. `docs` — documentation site with sidebar navigation, search (client-side with a generated JSON index), breadcrumbs, prev/next links. Inspired by Docusaurus/VitePress.
> 3. `portfolio` — single-page portfolio with project cards, about section, contact info. Responsive grid layout.
> 4. `blank` — absolute minimum: one page, one template, config file.
>
> Each starter should build successfully with `mythic init --template <name> mysite && cd mysite && mythic build`. Include a screenshot (placeholder text is fine). Write a `starters/README.md` describing each.

**Acceptance criteria:**
- All 4 starters build without errors
- `mythic init --template blog mysite` copies the correct starter
- Blog starter: has index page with post list, individual post pages, tag pages, RSS feed
- Docs starter: has sidebar nav, breadcrumbs, prev/next navigation, client-side search
- Portfolio starter: responsive grid layout, looks good on mobile
- Blank starter: absolute minimum files, builds to a single page
- Each starter has a `README.md` with customization instructions

---

### Iteration 11.3 — Documentation site

**Prompt for Claude Code:**
> Build the official Mythic documentation site using Mythic itself (dogfooding). Use the `docs` starter as a base. Create content for:
> 1. **Getting Started**: installation, quickstart, project structure
> 2. **Content**: markdown features, frontmatter, shortcodes, data files
> 3. **Templates**: Tera guide, Handlebars guide, template context reference
> 4. **Configuration**: complete `mythic.toml` reference with all options
> 5. **Assets**: image pipeline, CSS/JS processing, Sass
> 6. **Deployment**: GitHub Pages (with Action), Netlify, Vercel, Cloudflare Pages
> 7. **Migration**: guides for Jekyll, Hugo, Eleventy
> 8. **Plugins**: hook reference, Rhai scripting guide, examples
> 9. **Performance**: benchmarks, optimization tips
> 10. **API Reference**: auto-generated from Rust doc comments
>
> Host this at `mythic.dev` (or similar) via GitHub Pages using the Mythic Action.

**Acceptance criteria:**
- Docs site builds with Mythic
- All 10 sections have substantive content (not just placeholders)
- Navigation works (sidebar, breadcrumbs, prev/next)
- Client-side search works across all docs
- Deployed via the Mythic GitHub Action
- Mobile responsive
- Syntax-highlighted code examples throughout

---

## Summary: Iteration Dependency Graph

```
Phase 0 (scaffolding)
  └→ Phase 1 (MVP pipeline) — iterations are sequential
       └→ Phase 2 (incremental + dev server)
       └→ Phase 3 (assets) — can run parallel with Phase 2
       └→ Phase 4 (data + taxonomies)
            └→ Phase 5 (shortcodes + highlighting)
                 └→ Phase 6 (plugins)
  Phase 7 (migration) — can start after Phase 1
  Phase 8 (SEO + quality) — can start after Phase 4
  Phase 9 (multi-engine + i18n) — can start after Phase 4
  Phase 10 (performance + distribution) — after Phase 5
       └→ Phase 11 (GitHub Pages + docs) — final phase
```

**Parallel workstreams after Phase 1:**
- Stream A: Phase 2 → Phase 5 → Phase 6 → Phase 10
- Stream B: Phase 3 → Phase 8
- Stream C: Phase 4 → Phase 9
- Stream D: Phase 7 (independent)
- Final: Phase 11 (depends on all others)
