---
title: "Introduction to Mythic"
---

# Mythic

Mythic is a fast, flexible static site generator built in Rust. It transforms your Markdown content and templates into a complete, deployable website in milliseconds.

## Why Mythic?

Static site generators have been around for years, but most make you choose between speed and flexibility. Mythic refuses that tradeoff. It delivers sub-second builds for large sites while giving you the template engine, plugin system, and asset pipeline you actually need.

## Key Features

### Blazing Fast Builds

Mythic is written in Rust and uses parallel processing throughout the build pipeline. A site with 10,000 pages builds in under 2 seconds on typical hardware. Incremental builds are even faster, rebuilding only the pages affected by your changes.

```
$ mythic build
  Loaded 10,247 pages in 184ms
  Rendered templates in 612ms
  Processed assets in 340ms
  Built site in 1.14s
```

### Multi-Engine Templates

Choose between **Tera** and **Handlebars** for your templates, or use both in the same project. Mythic auto-detects the engine from file extensions:

```
templates/
  base.tera.html      # Tera template
  post.hbs.html       # Handlebars template
  page.tera.html      # Tera template
```

### Incremental Builds

Mythic tracks file dependencies and only rebuilds what changed. Edit a Markdown file and only that page re-renders. Edit a base template and Mythic knows which pages depend on it.

### Live Reload

The development server watches your files and reloads the browser automatically when content, templates, styles, or scripts change. No configuration required.

```
$ mythic serve
  Server running at http://localhost:3000
  Watching for changes...
```

### Plugin System

Extend Mythic with Rust plugins or lightweight Rhai scripts. Plugins hook into every stage of the build pipeline: content loading, template rendering, asset processing, and output writing.

```rust
use mythic_plugin::prelude::*;

pub struct WordCountPlugin;

impl Plugin for WordCountPlugin {
    fn on_page(&self, page: &mut Page) -> Result<()> {
        let count = page.content.split_whitespace().count();
        page.extra.insert("word_count".into(), count.into());
        Ok(())
    }
}
```

### Asset Pipeline

Mythic handles your images, CSS, and JavaScript out of the box. It compiles Sass/SCSS, bundles and minifies JavaScript, optimizes images, and adds content hashes for cache busting.

### Taxonomies

Built-in support for tags, categories, and custom taxonomies. Mythic generates listing pages and feeds for each taxonomy term automatically.

### Internationalization

First-class support for multilingual sites. Organize content by locale, generate hreflang tags, and access translations in templates with the `t()` function.

## How It Works

Mythic follows a simple pipeline:

1. **Load** content from Markdown files with YAML/TOML frontmatter
2. **Parse** Markdown into HTML with syntax highlighting, footnotes, and shortcodes
3. **Render** templates with page content, site data, and taxonomy information
4. **Process** assets through the image, CSS, and JS pipelines
5. **Write** the final HTML, assets, and feeds to the output directory

## Getting Started

Install Mythic and create your first site in under a minute:

```bash
cargo install mythic
mythic init my-site
cd my-site
mythic serve
```

Read the [Installation Guide](/getting-started/installation/) to get started, or jump to the [Quickstart](/getting-started/quickstart/) if you already have Mythic installed.

## Migrating from Another Generator

Mythic includes built-in migration tools for Jekyll, Hugo, and Eleventy. See the [Migration Guides](/migration/) for details.
