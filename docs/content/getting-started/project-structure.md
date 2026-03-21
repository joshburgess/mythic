---
title: "Project Structure"
---

# Project Structure

A Mythic project follows a well-defined directory layout. Every directory has a clear purpose, and Mythic uses this structure to determine how to process your files.

## Overview

```
my-site/
  mythic.toml            # Site configuration
  content/               # Markdown content
    index.md
    about.md
    blog/
      _dir.yaml          # Section defaults
      first-post.md
      second-post.md
  templates/             # Template files (Tera or Handlebars)
    base.tera.html
    page.tera.html
    blog.tera.html
    partials/
      header.tera.html
      footer.tera.html
  _data/                 # Global data files
    site.yaml
    nav.toml
    authors.json
  static/                # Static assets (copied as-is)
    favicon.ico
    robots.txt
    images/
      logo.png
  styles/                # CSS and SCSS source files
    main.scss
    _variables.scss
  scripts/               # JavaScript source files
    main.js
  plugins/               # Local plugin scripts
    reading-time.rhai
  public/                # Build output (generated)
```

## mythic.toml

The configuration file at the project root controls all aspects of your site. It defines the site title, base URL, template engine, taxonomy configuration, and more.

```toml
[site]
title = "My Site"
base_url = "https://example.com"
language = "en"

[build]
output = "public"
template_engine = "tera"
```

See the [Configuration Reference](/configuration/reference/) for all available options.

## content/

This is where your Markdown content lives. Each `.md` file becomes a page on your site. The directory structure maps directly to URL paths:

| File Path                        | URL                      |
|----------------------------------|--------------------------|
| `content/index.md`              | `/`                      |
| `content/about.md`              | `/about/`                |
| `content/blog/first-post.md`    | `/blog/first-post/`      |
| `content/docs/getting-started.md` | `/docs/getting-started/` |

### Section Directories

Any directory inside `content/` can include a `_dir.yaml` (or `_dir.toml` / `_dir.json`) file to set defaults for all pages in that section:

```yaml
# content/blog/_dir.yaml
layout: blog
author: "Default Author"
```

Every page in `content/blog/` inherits these values unless overridden in its own frontmatter.

### Index Pages

A file named `index.md` in any directory becomes the index page for that section. For example, `content/blog/index.md` renders at `/blog/`.

## templates/

Template files control how your content is rendered into HTML. Mythic supports both Tera (`.tera.html`) and Handlebars (`.hbs.html`) templates.

```
templates/
  base.tera.html       # Base layout
  page.tera.html       # Default page template
  blog.tera.html       # Blog post template
  taxonomy.tera.html   # Taxonomy listing template
  partials/
    header.tera.html
    footer.tera.html
    sidebar.tera.html
```

Template resolution follows this order:

1. The `layout` specified in the page's frontmatter
2. A template matching the content section name (e.g., `blog.tera.html` for pages in `content/blog/`)
3. The default `page.tera.html`

### Partials

Templates in the `partials/` subdirectory can be included by other templates. They cannot be used as standalone page layouts.

## _data/

Global data files accessible in all templates via the `data` variable. Mythic supports YAML, TOML, and JSON formats.

```
_data/
  site.yaml          -> {{ data.site }}
  nav.toml           -> {{ data.nav }}
  authors.json       -> {{ data.authors }}
  social/
    links.yaml       -> {{ data.social.links }}
```

The filename (without extension) becomes the key. Subdirectories create nested namespaces.

## static/

Files in `static/` are copied directly to the output directory without any processing. Use this for files that should be served as-is:

- `favicon.ico`
- `robots.txt`
- Pre-built assets
- Downloaded libraries
- Files that need exact paths (e.g., verification files)

A file at `static/images/logo.png` will be available at `/images/logo.png` in your built site.

## styles/

CSS and SCSS files go here. Mythic compiles SCSS to CSS, handles imports, and can minify the output for production builds.

```
styles/
  main.scss           # Entry point
  _variables.scss     # Sass partial (not compiled standalone)
  _mixins.scss        # Sass partial
  components/
    _buttons.scss
    _nav.scss
```

Files prefixed with `_` are treated as partials and are not compiled on their own. They are meant to be imported by other stylesheets.

The compiled CSS is available in templates via `{{ assets.css }}`.

## scripts/

JavaScript source files live here. Mythic bundles and optionally minifies them for production.

```
scripts/
  main.js             # Entry point
  utils.js
  components/
    modal.js
    dropdown.js
```

The bundled JavaScript is available in templates via `{{ assets.js }}`.

## plugins/

Local Rhai scripts or Rust plugin configuration. These extend Mythic's build pipeline with custom behavior.

```
plugins/
  reading-time.rhai
  toc-numbers.rhai
```

See [Plugins](/features/plugins/) for details on writing and configuring plugins.

## public/ (Output)

The generated output directory. This is created by `mythic build` and contains the fully rendered, deployable site. Do not edit files here directly; they will be overwritten on the next build.

Add `public/` to your `.gitignore`:

```
# .gitignore
public/
```
