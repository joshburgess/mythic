---
title: "Plugins"
---

# Plugins

Mythic's plugin system lets you extend every stage of the build pipeline. Write plugins in Rust for maximum performance or use Rhai scripts for quick customization without recompiling.

## Plugin Hooks

Plugins can hook into these build stages:

| Hook              | When it runs                                   | Use case                              |
|-------------------|------------------------------------------------|---------------------------------------|
| `on_load`         | After content files are read                   | Transform raw content                 |
| `on_page`         | After frontmatter is parsed, before rendering  | Add computed fields, filter pages     |
| `on_render`       | After template rendering                       | Post-process HTML                     |
| `on_asset`        | During asset processing                        | Custom asset transformations          |
| `on_finish`       | After the build completes                      | Generate search indexes, reports      |

## Rust Plugins

### The Plugin Trait

Create a Rust crate that implements the `Plugin` trait:

```rust
use mythic_plugin::prelude::*;

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    fn on_page(&self, page: &mut Page) -> Result<()> {
        // Modify page data before rendering
        Ok(())
    }
}

// Register the plugin
mythic_plugin::register!(MyPlugin);
```

### ReadingTimePlugin Example

A complete plugin that calculates reading time:

```rust
use mythic_plugin::prelude::*;

pub struct ReadingTimePlugin {
    words_per_minute: usize,
}

impl ReadingTimePlugin {
    pub fn new(wpm: usize) -> Self {
        Self { words_per_minute: wpm }
    }
}

impl Default for ReadingTimePlugin {
    fn default() -> Self {
        Self::new(200)
    }
}

impl Plugin for ReadingTimePlugin {
    fn name(&self) -> &str {
        "reading-time"
    }

    fn on_page(&self, page: &mut Page) -> Result<()> {
        let word_count = page.raw_content.split_whitespace().count();
        let reading_time = (word_count + self.words_per_minute - 1) / self.words_per_minute;

        page.extra.insert("word_count".into(), word_count.into());
        page.extra.insert("reading_time".into(), reading_time.into());

        Ok(())
    }
}

mythic_plugin::register!(ReadingTimePlugin);
```

Use the computed values in templates:

```html
<p>{{ page.extra.word_count }} words &middot; {{ page.extra.reading_time }} min read</p>
```

### Search Index Plugin Example

Generate a search index at build time:

```rust
use mythic_plugin::prelude::*;
use serde::Serialize;

pub struct SearchIndexPlugin;

#[derive(Serialize)]
struct SearchEntry {
    title: String,
    path: String,
    content: String,
    tags: Vec<String>,
}

impl Plugin for SearchIndexPlugin {
    fn name(&self) -> &str {
        "search-index"
    }

    fn on_finish(&self, site: &Site, output: &Path) -> Result<()> {
        let entries: Vec<SearchEntry> = site.pages.iter()
            .filter(|p| !p.draft)
            .map(|p| SearchEntry {
                title: p.title.clone(),
                path: p.path.clone(),
                content: p.plain_text.clone(),
                tags: p.tags.clone(),
            })
            .collect();

        let json = serde_json::to_string(&entries)?;
        let index_path = output.join("search-index.json");
        std::fs::write(index_path, json)?;

        Ok(())
    }
}

mythic_plugin::register!(SearchIndexPlugin);
```

### Installing Rust Plugins

Add the plugin crate to your project:

```toml
# mythic.toml
[plugins.reading-time]
crate = "mythic-plugin-reading-time"
version = "0.2"
```

Or reference a local path:

```toml
[plugins.my-plugin]
path = "plugins/my-plugin"
```

## Rhai Scripting

For simpler customizations, write plugins as Rhai scripts. Rhai is a lightweight scripting language embedded in Mythic.

### Creating a Rhai Plugin

Place `.rhai` files in the `plugins/` directory:

```
plugins/
  reading-time.rhai
  last-modified.rhai
```

### Reading Time in Rhai

```rhai
// plugins/reading-time.rhai

fn on_page(page) {
    let words = page.raw_content.split(" ").len();
    let minutes = (words + 199) / 200;

    page.extra.word_count = words;
    page.extra.reading_time = minutes;
}
```

### Last Modified Date in Rhai

```rhai
// plugins/last-modified.rhai

fn on_page(page) {
    if page.updated == () {
        page.extra.show_updated = false;
    } else {
        page.extra.show_updated = true;
        page.extra.days_since_update = (now() - page.updated).days();
    }
}
```

### External Link Processing in Rhai

```rhai
// plugins/external-links.rhai

fn on_render(html, page) {
    // Add noopener to external links
    let result = html.replace(
        "target=\"_blank\"",
        "target=\"_blank\" rel=\"noopener noreferrer\""
    );
    result
}
```

### Enabling Rhai Plugins

Rhai plugins in the `plugins/` directory are loaded automatically. To configure them or load from a custom path:

```toml
[plugins.reading-time]
path = "plugins/reading-time.rhai"

[plugins.custom-script]
path = "custom/my-script.rhai"
```

## Built-in Plugins

Mythic ships with several plugins that can be enabled in configuration:

### reading_time

Adds `word_count` and `reading_time` to page extra data.

```toml
[plugins]
reading_time = true
```

### search

Generates a JSON search index for client-side search.

```toml
[plugins.search]
enable = true
index_content = true
index_title = true
index_tags = true
```

### sitemap

Automatically generates `sitemap.xml`. Enabled by default.

```toml
[sitemap]
enable = true
```

### feed

Generates Atom feeds. Configure per-taxonomy feeds in the taxonomies section.

```toml
[site]
generate_feed = true
feed_limit = 20
```

## Plugin Execution Order

Plugins run in the order they appear in `mythic.toml`. If plugin B depends on data from plugin A, list A first:

```toml
[plugins]
reading_time = true       # Runs first, adds word_count

[plugins.estimated-cost]  # Runs second, can use word_count
path = "plugins/cost.rhai"
```

## Computed Frontmatter

Mythic supports computed frontmatter fields using inline Rhai expressions. This lets you derive page metadata from other fields without writing a full plugin.

### Syntax

In your page frontmatter, set a field under `extra` to a string prefixed with `rhai:`:

```yaml
---
title: "My Long Article"
date: 2026-03-15
extra:
  reading_time: "rhai: (word_count + 199) / 200"
  is_long: "rhai: word_count > 2000"
  upper_slug: "rhai: slug.to_upper()"
---
```

When Mythic processes the page, it evaluates each `rhai:` expression and replaces the string with the computed result.

### Available Variables

The following variables are available inside computed frontmatter expressions:

| Variable     | Type    | Description                                |
|--------------|---------|--------------------------------------------|
| `word_count` | Integer | Number of words in the page content        |
| `slug`       | String  | The URL slug of the page                   |
| `title`      | String  | The page title from frontmatter            |
| `date`       | String  | The date as an ISO 8601 string, or `""`    |
| `has_date`   | Boolean | Whether the page has a date set            |

### Examples

#### Reading Time

```yaml
extra:
  reading_time: "rhai: (word_count + 199) / 200"
```

This calculates reading time assuming 200 words per minute, rounding up.

#### Long Content Flag

```yaml
extra:
  is_long: "rhai: word_count > 2000"
```

Use this in templates to conditionally show a table of contents or a progress bar:

```html
{% if page.extra.is_long %}
<nav class="toc">{{ toc | safe }}</nav>
{% endif %}
```

#### Slug Transformations

```yaml
extra:
  upper_slug: "rhai: slug.to_upper()"
  slug_with_prefix: "rhai: \"post-\" + slug"
```

#### Conditional Date Display

```yaml
extra:
  show_date: "rhai: has_date"
  date_label: "rhai: if has_date { \"Published: \" + date } else { \"Undated\" }"
```

### Error Handling

If a computed frontmatter expression contains a syntax error or fails at runtime, Mythic emits a warning but does not fail the build. The field is left as the raw string (including the `rhai:` prefix) so you can spot it in the rendered output.

```
  Warning: content/blog/my-post.md: computed field "reading_time" failed: variable 'word_countt' not found
```

This ensures that a typo in one expression does not prevent the rest of your site from building.

## Plugin API Reference

### Page Object

Fields available on the `page` object in plugin hooks:

| Field            | Type         | Writable | Description                    |
|------------------|-------------|----------|--------------------------------|
| `title`          | String      | Yes      | Page title                     |
| `date`           | DateTime    | Yes      | Publication date               |
| `updated`        | DateTime    | Yes      | Last updated date              |
| `draft`          | Boolean     | Yes      | Draft status                   |
| `path`           | String      | No       | URL path                       |
| `slug`           | String      | Yes      | URL slug                       |
| `raw_content`    | String      | No       | Original Markdown content      |
| `plain_text`     | String      | No       | Content with all markup removed|
| `tags`           | Vec<String> | Yes      | Tag list                       |
| `categories`     | Vec<String> | Yes      | Category list                  |
| `extra`          | Map         | Yes      | Custom data map                |
| `locale`         | String      | Yes      | Page locale                    |

### Site Object

Available in `on_finish`:

| Field     | Type        | Description              |
|-----------|-------------|--------------------------|
| `title`   | String      | Site title               |
| `pages`   | Vec<Page>   | All pages                |
| `config`  | Config      | Full site configuration  |
