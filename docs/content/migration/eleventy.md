---
title: "Migrating from Eleventy"
---

# Migrating from Eleventy

Mythic can migrate Eleventy (11ty) sites, converting Nunjucks templates, Liquid templates, data files, and configuration. Since Nunjucks and Tera share very similar syntax, most templates convert cleanly.

## Running the Migration

```bash
mythic migrate --from eleventy --source /path/to/11ty-site --output my-mythic-site
```

The `--from 11ty` alias is also accepted:

```bash
mythic migrate --from 11ty --source . --output ../mythic-site
```

### Options

| Flag         | Default | Description                                    |
|--------------|---------|------------------------------------------------|
| `--source`   | `.`     | Path to the Eleventy project                   |
| `--output`   | `./out` | Path for the new Mythic project                |
| `--dry-run`  | `false` | Preview changes without writing files          |
| `--verbose`  | `false` | Show detailed conversion output                |

## What Gets Converted

### Content Files

| Eleventy                        | Mythic                              |
|---------------------------------|-------------------------------------|
| Markdown files (`.md`)         | `content/` (copied, structure preserved) |
| Nunjucks content (`.njk`)      | `content/*.md` or `templates/`      |
| Liquid content (`.liquid`)     | `content/*.md` or `templates/`      |
| HTML content (`.html`)        | `content/` or `templates/`          |

Content files with frontmatter and body text are placed in `content/`. Pure template files (layouts, includes) go to `templates/`.

### Configuration

Eleventy's `.eleventy.js` (or `eleventy.config.js`) is parsed to extract directory configuration:

```javascript
// Eleventy .eleventy.js
module.exports = function(eleventyConfig) {
    eleventyConfig.addPassthroughCopy("assets");
    eleventyConfig.addCollection("posts", function(collectionApi) {
        return collectionApi.getFilteredByGlob("src/posts/*.md");
    });

    return {
        dir: {
            input: "src",
            output: "_site",
            includes: "_includes",
            data: "_data"
        }
    };
};
```

```toml
# Mythic mythic.toml (converted)
[site]
title = ""
base_url = ""

[build]
output = "public"
template_engine = "tera"
```

The migration tool extracts directory paths to find your content, includes, and data. JavaScript logic (collections, filters, plugins) is flagged for manual conversion.

### Template Includes

| Eleventy                    | Mythic                               |
|-----------------------------|--------------------------------------|
| `_includes/base.njk`       | `templates/base.tera.html`           |
| `_includes/header.njk`     | `templates/partials/header.tera.html`|
| `_includes/footer.njk`     | `templates/partials/footer.tera.html`|

### Data Files

| Eleventy                     | Mythic                  |
|------------------------------|-------------------------|
| `_data/site.json`           | `_data/site.json`       |
| `_data/nav.yaml`            | `_data/nav.yaml`        |
| `_data/metadata.js`         | Not converted (warning) |
| `posts/posts.json`          | `content/posts/_dir.yaml`|

Static data files (JSON, YAML) are copied directly. JavaScript data files are flagged with a warning.

### Directory Data Files

Eleventy's directory data files (e.g., `posts/posts.json`) are converted to Mythic's `_dir.yaml`:

```json
// Eleventy: posts/posts.json
{
    "layout": "post",
    "tags": "posts",
    "permalink": "/blog/{{ page.fileSlug }}/"
}
```

```yaml
# Mythic: content/posts/_dir.yaml (converted)
layout: blog
```

### Static Assets

| Eleventy                     | Mythic                    |
|------------------------------|---------------------------|
| Passthrough copy directories | `static/`                 |
| CSS files                    | `styles/` or `static/`   |
| JS files                     | `scripts/` or `static/`  |

Directories registered with `addPassthroughCopy` are moved to `static/`.

## Nunjucks to Tera Conversion

Nunjucks and Tera share a common Jinja2 heritage, so most syntax is identical. The migration tool handles the differences automatically.

### Identical Syntax (No Changes Needed)

These constructs work the same in both Nunjucks and Tera:

```html
<!-- Variables -->
{{ page.title }}
{{ data.site.author }}

<!-- Filters -->
{{ title | upper }}
{{ name | lower }}
{{ text | trim }}
{{ text | replace("old", "new") }}

<!-- Control structures -->
{% for item in items %}
  {{ item.name }}
{% endfor %}

{% if condition %}
  ...
{% elif other %}
  ...
{% else %}
  ...
{% endif %}

<!-- Template inheritance -->
{% extends "base.html" %}
{% block content %}...{% endblock %}

<!-- Includes -->
{% include "partials/header.html" %}

<!-- Variables -->
{% set name = "value" %}

<!-- Comments -->
{# This is a comment #}
```

### Syntax Differences

| Nunjucks                                | Tera                                          |
|-----------------------------------------|-----------------------------------------------|
| `{{ data \| dump }}`                    | `{{ data \| json_encode() }}`                |
| `{{ data \| dump(2) }}`                | `{{ data \| json_encode() }}`                |
| `{{ loop.index0 }}`                    | `{{ loop.index0 }}`                          |
| `{% asyncEach %}`                      | `{% for %}` (no async)                       |
| `{{ caller() }}`                       | Not supported (use blocks)                    |
| `{% macro name(args) %}`              | `{% macro name(args) %}`                     |
| `{% from "m.html" import name %}`     | `{% import "m.html" as m %}` then `m::name()`|

### Filter Differences

| Nunjucks Filter           | Tera Equivalent                         |
|---------------------------|-----------------------------------------|
| `dump`                    | `json_encode()`                         |
| `safe`                    | `safe`                                  |
| `length`                  | `length`                                |
| `reverse`                 | `reverse`                               |
| `first`                   | `first`                                 |
| `last`                    | `last`                                  |
| `join(",")`               | `join(sep=",")`                         |
| `sort(false, true, "k")` | `sort_by(attribute="k")`               |
| `groupby("key")`         | `group_by(attribute="key")`            |
| `striptags`               | `striptags`                             |
| `truncate(100)`           | `truncate(length=100)`                  |
| `urlencode`               | `urlencode`                             |
| `dictsort`                | Not available (sort in data file)       |
| `random`                  | Not available (deterministic builds)    |

## JavaScript Data File Warnings

JavaScript data files (`.js` files in `_data/`) cannot be automatically converted because they contain executable code:

```javascript
// _data/metadata.js (Eleventy)
module.exports = {
    title: "My Site",
    url: process.env.URL || "http://localhost:8080",
    author: {
        name: "Jane Doe",
        email: "jane@example.com"
    }
};
```

The migration tool flags these files and suggests conversion:

```
WARNING: Cannot convert JavaScript data file: _data/metadata.js
  -> Convert to static YAML/JSON manually
  -> For dynamic data, use a Rhai plugin
```

Convert to a static data file:

```yaml
# _data/metadata.yaml
title: "My Site"
url: "https://example.com"
author:
  name: "Jane Doe"
  email: "jane@example.com"
```

For dynamic data that depends on environment variables, use Mythic's environment variable support:

```bash
MYTHIC_BASE_URL="https://example.com" mythic build
```

### Computed Data

Eleventy's computed data feature allows deriving values from other data:

```javascript
// Eleventy computed data
module.exports = {
    eleventyComputed: {
        permalink: data => `/blog/${data.page.fileSlug}/`,
        readableDate: data => formatDate(data.page.date)
    }
};
```

In Mythic, handle this with:

- **Slug overrides** in frontmatter for custom URLs
- **Plugins** (Rhai or Rust) for computed values
- **Template logic** for display formatting

## What Requires Manual Conversion

### Custom Nunjucks Filters

Filters registered in `.eleventy.js` need Tera equivalents or Rhai plugins:

```javascript
// Eleventy
eleventyConfig.addFilter("readableDate", (dateObj) => {
    return DateTime.fromJSDate(dateObj).toFormat("dd LLLL yyyy");
});
```

In Mythic, use Tera's built-in date filter:

```html
{{ page.date | date(format="%d %B %Y") }}
```

For custom filters with no Tera equivalent, write a Rhai plugin:

```rhai
// plugins/custom-filters.rhai
fn on_page(page) {
    // Add computed values to page.extra
    page.extra.custom_value = "computed result";
}
```

### Eleventy Collections

Eleventy collections defined in JavaScript need to be replaced with Mythic's section and taxonomy system:

```javascript
// Eleventy
eleventyConfig.addCollection("tagList", function(collectionApi) {
    let tags = new Set();
    collectionApi.getAll().forEach(item => {
        (item.data.tags || []).forEach(tag => tags.add(tag));
    });
    return [...tags].sort();
});
```

In Mythic, tags are built-in:

```toml
# mythic.toml
[taxonomies]
tags = { feed = true, paginate = 10 }
```

Access in templates:

```html
{% for tag in site.taxonomies.tags %}
  <a href="/tags/{{ tag.slug }}/">{{ tag.name }}</a>
{% endfor %}
```

### Eleventy Plugins

Common Eleventy plugins and their Mythic equivalents:

| Eleventy Plugin                   | Mythic Equivalent                        |
|-----------------------------------|------------------------------------------|
| `@11ty/eleventy-plugin-rss`      | Built-in: `generate_feed = true`         |
| `@11ty/eleventy-plugin-syntaxhighlight` | Built-in: code block highlighting |
| `eleventy-plugin-toc`            | Built-in: `{{ toc }}`                    |
| `@11ty/eleventy-img`            | Built-in: image pipeline                 |
| `eleventy-plugin-reading-time`   | Built-in plugin: `reading_time = true`   |
| `@11ty/eleventy-navigation`     | Data files and template logic            |

### Pagination

Eleventy's pagination feature for generating pages from data needs manual conversion:

```yaml
# Eleventy pagination frontmatter
---
pagination:
  data: collections.posts
  size: 10
---
```

In Mythic, use taxonomy pagination:

```toml
# mythic.toml
[taxonomies]
tags = { paginate = 10 }
```

For paginating section listings, configure pagination in the section:

```yaml
# content/blog/_dir.yaml
paginate: 10
```

## Step-by-Step Migration

1. **Run the migration:**
   ```bash
   mythic migrate --from eleventy --source . --output ../mythic-site --verbose
   ```

2. **Review warnings** about JavaScript data files and custom filters.

3. **Convert JS data files** to YAML or JSON.

4. **Review templates** in `templates/`. Most Nunjucks syntax transfers cleanly, but check for custom filters.

5. **Build and test:**
   ```bash
   cd ../mythic-site
   mythic build
   mythic serve
   ```

6. **Replace custom filters** with Tera built-ins or Rhai plugins.

7. **Compare pages** with your Eleventy site to verify correctness.
