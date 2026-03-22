# Mythic

A fast, batteries-included static site generator written in Rust.

## Features

- **Fast builds** - Parallel markdown rendering, incremental rebuilds (10k pages in 200ms when unchanged)
- **Live reload** - Dev server with WebSocket-based hot reload, CSS injection without full page refresh
- **Multi-engine templates** - Tera and Handlebars side by side in the same project
- **Asset pipeline** - Image optimization (WebP), CSS/JS bundling and minification, Sass/SCSS compilation
- **Syntax highlighting** - Built-in via syntect with configurable themes and line numbers
- **Shortcodes** - Custom reusable components as Tera templates with paired and self-closing syntax
- **Data system** - YAML/TOML/JSON data files with Eleventy-style directory data cascade
- **Taxonomies** - Tags, categories, and custom taxonomies with listing and term pages
- **Atom + RSS feeds** - Site-wide and per-taxonomy feeds in both formats
- **i18n** - Locale directories, hreflang tags, translation files
- **Plugin system** - Rust trait-based hooks plus Rhai scripting for user plugins
- **SEO tools** - Sitemap, robots.txt, link checker, heading hierarchy validation
- **Search** - JSON search index generation for client-side search (Fuse.js, Lunr.js)
- **Pagination** - Paginated taxonomy and listing pages with full paginator context
- **Redirects** - Frontmatter `aliases` generate HTML redirect files with canonical links
- **404 pages** - `content/404.md` automatically renders as `404.html` for static hosts
- **Migration tools** - Import from Jekyll, Hugo, or Eleventy

## Install

### From source

```bash
cargo install --path crates/mythic-cli
```

### From binary

```bash
curl -fsSL https://raw.githubusercontent.com/joshburgess/mythic/main/install.sh | sh
```

## Quickstart

```bash
# Create a new site
mythic init my-site --template blog

# Start the dev server
cd my-site
mythic serve
```

Open http://localhost:3000 in your browser. Edit files and see changes instantly.

## Commands

```
mythic init <name>              Create a new site (--template: blank, blog, docs, portfolio)
mythic new <type> "Title"       Create a new content file (--draft)
mythic build                    Build the site (--clean, --drafts, --profile, --quiet)
mythic serve                    Dev server with live reload (--port, --open)
mythic check                    Validate links, images, and heading hierarchy
mythic list                     List all content pages with dates and slugs (--drafts)
mythic clean                    Delete the output directory
mythic migrate --from <ssg>     Import from jekyll, hugo, or eleventy
mythic completions <shell>      Generate shell completions (bash, zsh, fish, powershell)
mythic --version                Show version
```

## Project Structure

```
my-site/
  mythic.toml          # Site configuration
  content/             # Markdown content with frontmatter
  templates/           # Tera (.html) and Handlebars (.hbs) templates
  _data/               # YAML/TOML/JSON data files
  static/              # Static assets (copied as-is)
  styles/              # CSS and SCSS files (bundled + minified)
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

[toc]
min_level = 2
max_level = 4
```

See the [configuration reference](docs/content/configuration/reference.md) for all options.

## Template Context

Templates receive these variables:

| Variable | Description |
|----------|-------------|
| `{{ page.title }}` | Page title from frontmatter |
| `{{ page.date }}` | Page date |
| `{{ page.tags }}` | Tag list |
| `{{ page.extra }}` | Custom frontmatter fields |
| `{{ content \| safe }}` | Rendered HTML content |
| `{{ site.title }}` | Site title from config |
| `{{ site.base_url }}` | Base URL from config |
| `{{ toc }}` | Table of contents entries |
| `{{ data.paginator }}` | Pagination context (on taxonomy pages) |
| `{{ assets.css_path }}` | Hashed CSS bundle path |
| `{{ assets.js_path }}` | Hashed JS bundle path |
| `{{ data }}` | Data from `_data/` files |

## Performance

Benchmarked against Hugo and Eleventy on identical synthetic sites (Apple M-series, release build):

| Pages  | Mythic   | Mythic (flat) | Hugo     | Eleventy  |
|-------:|---------:|--------------:|---------:|----------:|
| 1,000  | 162ms    | —             | 98ms     | 290ms     |
| 10,000 | 1,822ms  | 1,338ms       | 1,718ms  | ~5,200ms  |

**Incremental rebuilds** (10k pages, no changes): Mythic **174ms**, Hugo 1,718ms, Eleventy ~3,500ms — **9.9x faster than Hugo**.

Enable `ugly_urls = true` for flat output (`slug.html` instead of `slug/index.html`) to beat Hugo by 22% on cold builds.

See [BENCHMARKS.md](BENCHMARKS.md) for full methodology, pipeline profiling, and optimization history.

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

### Other platforms

Works with any static hosting. See deployment docs for [Netlify, Vercel, and Cloudflare Pages](docs/content/deployment/other.md).

## Architecture

Cargo workspace with six crates:

| Crate | Purpose |
|-------|---------|
| `mythic-core` | Config, content discovery, build pipeline, caching, plugins |
| `mythic-markdown` | Frontmatter, pulldown-cmark rendering, shortcodes, TOC, syntax highlighting |
| `mythic-template` | Tera + Handlebars multi-engine rendering |
| `mythic-assets` | Image processing, CSS/JS bundling, Sass compilation |
| `mythic-server` | Dev server (axum), file watcher, WebSocket live reload |
| `mythic-cli` | CLI binary (clap) |

## License

MIT
