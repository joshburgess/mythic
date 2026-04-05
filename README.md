# Mythic

A fast, batteries-included static site generator written in Rust. Faster than Hugo on cold builds, 23x faster on incremental rebuilds.

## Getting Started

### Install

From crates.io:

```bash
cargo install mythic-cli
```

From binary (Linux/macOS):

```bash
curl -fsSL https://raw.githubusercontent.com/joshburgess/mythic/main/install.sh | sh
```

From source:

```bash
git clone https://github.com/joshburgess/mythic.git
cargo install --path mythic/crates/mythic-cli
```

### Create a site

```bash
mythic init my-site --template blog
cd my-site
mythic serve
```

Open http://localhost:3000 and start editing. Changes appear instantly via live reload.

Starter templates: `blank`, `blog`, `docs`, `portfolio`, `minimal`.

## Why Mythic

### Fast

Mythic is faster than Hugo at every scale tested, and 2-3x faster than Eleventy:

| Pages  | Mythic   | Hugo     | Eleventy  |
|-------:|---------:|---------:|----------:|
| 1,000  | 150ms    | 171ms    | 300ms     |
| 5,000  | 740ms    | 851ms    | 1,510ms   |
| 10,000 | 1,614ms  | 2,925ms  | 3,860ms   |

Incremental rebuilds are where Mythic really shines. When no content has changed, Mythic skips rendering, templating, and writing entirely:

| Pages  | Mythic    | Hugo     | Eleventy  |
|-------:|----------:|---------:|----------:|
| 1,000  | **10ms**  | 171ms    | ~300ms    |
| 10,000 | **125ms** | 2,925ms  | ~3,860ms  |

See [BENCHMARKS.md](BENCHMARKS.md) for full methodology and optimization details.

### Batteries included

Everything you need for a production site, with no plugins or external tools required:

**Content** — Markdown with YAML/TOML frontmatter, syntax highlighting, shortcodes, table of contents, admonitions (`> [!NOTE]`), math rendering (KaTeX), and render hooks for links and images.

**Templates** — Tera, Handlebars, and MiniJinja side by side in the same project. Custom filters for reading time, word count, and Hugo-compatible helpers.

**Assets** — Image optimization with automatic WebP generation and responsive `<picture>` tags. CSS/JS bundling and minification. Sass/SCSS compilation. SRI integrity hashes.

**Data** — Load YAML, TOML, or JSON from `_data/` files. Directory data cascade. Fetch remote APIs at build time with filesystem caching. Computed frontmatter via Rhai expressions.

**SEO** — Sitemap, robots.txt, Schema.org JSON-LD, Atom + RSS + JSON Feed generation (site-wide and per-taxonomy), canonical URLs, and hreflang tags for i18n.

**Quality** — Build-time accessibility auditing (WCAG), link checking, heading hierarchy validation, and configurable content linting (word counts, required fields, orphan detection).

**Dev experience** — Dev server with WebSocket live reload, CSS injection without full page refresh, and error overlays. Config and template changes trigger automatic rebuilds.

### Extensible

- **Plugin system** — Rust trait-based hooks plus Rhai scripting for user-defined build logic
- **Taxonomies** — Tags, categories, and custom taxonomies with paginated listing pages
- **i18n** — Locale directories, translation files, hreflang tags
- **Migration tools** — Import existing sites from Jekyll, Hugo, or Eleventy
- **Custom output formats** — JSON API output alongside HTML

## Commands

```
mythic init <name>              Create a new site (--template blog)
mythic new <type> "Title"       Create a new content file (--draft)
mythic build                    Build the site (--clean, --drafts, --profile, --json)
mythic serve                    Dev server with live reload (--port, --open)
mythic watch                    Watch and rebuild without a server
mythic check                    Validate links, images, and headings
mythic list                     List all content pages (--drafts)
mythic clean                    Delete the output directory
mythic migrate --from <ssg>     Import from jekyll, hugo, or eleventy
mythic completions <shell>      Generate shell completions
```

## Project Structure

```
my-site/
  mythic.toml          # Site configuration
  content/             # Markdown content with frontmatter
  templates/           # Tera (.html), Handlebars (.hbs), and MiniJinja (.jinja) templates
  _data/               # YAML/TOML/JSON data files
  static/              # Static assets (copied as-is)
  styles/              # CSS/SCSS files (bundled + minified)
  scripts/             # JavaScript files (bundled + minified)
  shortcodes/          # Shortcode templates
  plugins/             # Rhai plugin scripts
  public/              # Build output (gitignored)
```

## Configuration

```toml
title = "My Site"
base_url = "https://example.com"

[[taxonomies]]
name = "tags"
slug = "tags"
feed = true

[feed]
title = "My Site Feed"
author = "Author Name"

[highlight]
theme = "base16-ocean.dark"
line_numbers = false
```

See the [configuration reference](docs/content/configuration/reference.md) for all options.

## Template Context

Templates receive these variables:

| Variable | Description |
|----------|-------------|
| `{{ page.title }}` | Page title from frontmatter |
| `{{ page.date }}` | Page date |
| `{{ page.slug }}` | Page slug |
| `{{ page.url }}` | Page URL path |
| `{{ page.tags }}` | Tag list |
| `{{ page.extra }}` | Custom frontmatter fields |
| `{{ content \| safe }}` | Rendered HTML content |
| `{{ site.title }}` | Site title from config |
| `{{ site.base_url }}` | Base URL from config |
| `{{ site.base_path }}` | URL path prefix (for subpath deploys) |
| `{{ toc }}` | Table of contents entries |
| `{{ assets.css_path }}` | Hashed CSS bundle path |
| `{{ assets.js_path }}` | Hashed JS bundle path |
| `{{ data }}` | Data from `_data/` files |
| `{{ get_pages() }}` | All pages (title, slug, url, date, tags) |
| `{{ get_sections() }}` | Pages grouped by content section |

## Deploy

### GitHub Pages

```yaml
# .github/workflows/deploy.yml
name: Deploy
on:
  push:
    branches: [main]
permissions:
  contents: read
  pages: write
  id-token: write
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deploy.outputs.page_url }}
    steps:
      - uses: actions/checkout@v4
      - uses: joshburgess/mythic/action@main
      - uses: actions/deploy-pages@v4
        id: deploy
```

Works with any static host. See deployment docs for [Netlify, Vercel, and Cloudflare Pages](docs/content/deployment/other.md).

## Architecture

Cargo workspace with six crates:

| Crate | Purpose |
|-------|---------|
| `mythic-core` | Config, content discovery, build pipeline, caching, plugins |
| `mythic-markdown` | Frontmatter, markdown rendering, shortcodes, syntax highlighting |
| `mythic-template` | Tera + Handlebars + MiniJinja multi-engine rendering |
| `mythic-assets` | Image processing, CSS/JS bundling, Sass compilation |
| `mythic-server` | Dev server (axum), file watcher, WebSocket live reload |
| `mythic-cli` | CLI binary (clap) |

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
