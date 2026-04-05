---
title: "Configuration Reference"
---

# Configuration Reference

Mythic is configured through a `mythic.toml` file at the root of your project. This page documents every available option with defaults.

## Complete Example

```toml
[site]
title = "My Site"
base_url = "https://example.com"
description = "A site built with Mythic"
language = "en"
generate_feed = true
feed_limit = 20

[build]
output = "public"
template_engine = "tera"
drafts = false
deep_merge_frontmatter = false
parallel = true

[server]
port = 3000
host = "127.0.0.1"
live_reload = true
open_browser = false

[markdown]
smart_punctuation = true
allow_html = true
heading_anchors = true
external_links_new_tab = true
syntax_theme = "onedark"

[assets]
sass = true
minify_css = true
minify_js = true
hash_filenames = true
image_widths = [640, 960, 1280, 1920]
image_quality = 85
image_format = "webp"

[taxonomies]
tags = { feed = true, paginate = 10 }
categories = { feed = true, paginate = 0 }

[sitemap]
enable = true
changefreq = "weekly"
priority = 0.5

[i18n]
default_locale = "en"
locales = ["en", "fr", "de", "ja"]

[plugins]
reading_time = true

[lint]
min_word_count = 100
max_word_count = 10000
required_fields = ["title", "date"]
require_tags = true
require_date = true

[[remote]]
name = "github_repos"
url = "https://api.github.com/users/myuser/repos"
ttl = 3600
```

## [site]

Core site metadata used in templates and feed generation.

| Option          | Type    | Default     | Description                                           |
|-----------------|---------|-------------|-------------------------------------------------------|
| `title`         | String  | `""`        | Site title, available as `site.title` in templates    |
| `base_url`      | String  | `""`        | Full base URL including protocol                      |
| `description`   | String  | `""`        | Site description for meta tags and feeds              |
| `language`      | String  | `"en"`      | Default language code (BCP 47)                        |
| `generate_feed` | Boolean | `false`     | Generate an Atom/RSS feed                             |
| `feed_limit`    | Integer | `20`        | Maximum number of items in the feed                   |
| `feed_filename` | String  | `"atom.xml"`| Output filename for the feed                          |

```toml
[site]
title = "My Blog"
base_url = "https://blog.example.com"
description = "Thoughts on Rust and web development"
language = "en"
generate_feed = true
feed_limit = 50
feed_filename = "feed.xml"
```

Mythic generates Atom (`feed.xml`), RSS 2.0 (`rss.xml`), and JSON Feed 1.1 (`feed.json`) automatically when feeds are enabled. Link to all three in your template:

```html
<link rel="alternate" type="application/atom+xml" href="/feed.xml">
<link rel="alternate" type="application/rss+xml" href="/rss.xml">
<link rel="alternate" type="application/feed+json" href="/feed.json">
```

> **Note:** The `feed.json` file follows the [JSON Feed 1.1](https://jsonfeed.org/version/1.1) specification, providing a JSON-based alternative to XML feeds. Many modern feed readers support JSON Feed natively.

## [build]

Controls the build process and output.

| Option                    | Type    | Default    | Description                                         |
|---------------------------|---------|------------|-----------------------------------------------------|
| `output`                  | String  | `"public"` | Output directory for the built site                 |
| `template_engine`         | String  | `"tera"`   | Default template engine (`"tera"`, `"handlebars"`, or `"minijinja"`)|
| `drafts`                  | Boolean | `false`    | Include draft pages in builds                       |
| `deep_merge_frontmatter`  | Boolean | `false`    | Deep merge `_dir.yaml` defaults with page frontmatter|
| `parallel`                | Boolean | `true`     | Use parallel processing during builds               |
| `clean`                   | Boolean | `true`     | Clean output directory before building              |
| `incremental`             | Boolean | `true`     | Enable incremental builds                           |
| `ugly_urls`               | Boolean | `false`    | Use flat output mode (`page.html` instead of `page/index.html`) |

### CLI Build Flags

| Flag | Description |
|------|-------------|
| `--clean` | Delete output directory before building |
| `--drafts` | Include draft pages |
| `--profile` | Print per-stage timing breakdown |
| `--quiet` | Suppress all output except errors |
| `--json` | Output build report as JSON (for CI scripting) |

```toml
[build]
output = "dist"
template_engine = "tera"
drafts = false
deep_merge_frontmatter = true
parallel = true
clean = true
incremental = true
ugly_urls = true
```

### Ugly URLs

When `ugly_urls = true`, Mythic writes each page as a flat file (e.g., `blog/my-post.html`) instead of the default "clean URL" style (`blog/my-post/index.html`). This reduces the total number of directories created during a build and can noticeably speed up builds on sites with thousands of pages. It also matches the URL scheme some hosting environments expect. Note that your internal links should include the `.html` extension when this mode is enabled.

### Config Validation

Mythic validates your `mythic.toml` on every build and emits a warning for any unrecognized keys. This helps catch typos and outdated options early. For example, if you write `template_engne` instead of `template_engine`, you will see:

```
  Warning: unrecognized config key `build.template_engne` in mythic.toml
```

No action is required to enable this behavior -- it runs automatically.

## [server]

Development server configuration.

| Option         | Type    | Default       | Description                                    |
|----------------|---------|---------------|------------------------------------------------|
| `port`         | Integer | `3000`        | Port number for the development server         |
| `host`         | String  | `"127.0.0.1"` | Host address to bind to                       |
| `live_reload`  | Boolean | `true`        | Enable live reload on file changes             |
| `open_browser` | Boolean | `false`       | Open the site in a browser on `mythic serve`   |

```toml
[server]
port = 8080
host = "0.0.0.0"
live_reload = true
open_browser = true
```

## [markdown]

Markdown processing options.

| Option                     | Type    | Default      | Description                                    |
|----------------------------|---------|--------------|------------------------------------------------|
| `smart_punctuation`        | Boolean | `true`       | Convert quotes, dashes, and ellipses           |
| `allow_html`               | Boolean | `true`       | Allow raw HTML in Markdown files               |
| `heading_anchors`          | Boolean | `true`       | Add `id` attributes to headings                |
| `external_links_new_tab`   | Boolean | `true`       | Add `target="_blank"` to external links        |
| `external_links_nofollow`  | Boolean | `false`      | Add `rel="nofollow"` to external links         |
| `syntax_theme`             | String  | `"onedark"`  | Syntax highlighting color theme                |
| `syntax_theme_path`        | String  | `""`         | Path to a custom `.tmTheme` file               |

```toml
[markdown]
smart_punctuation = true
allow_html = true
heading_anchors = true
external_links_new_tab = true
external_links_nofollow = false
syntax_theme = "github-dark"
```

## [assets]

Asset processing pipeline configuration.

| Option            | Type    | Default  | Description                                          |
|-------------------|---------|----------|------------------------------------------------------|
| `sass`            | Boolean | `true`   | Compile Sass/SCSS files                              |
| `minify_css`      | Boolean | `true`   | Minify CSS output in production builds               |
| `minify_js`       | Boolean | `true`   | Minify JavaScript output in production builds        |
| `hash_filenames`  | Boolean | `true`   | Add content hashes to output filenames               |
| `image_widths`    | Array   | `[640, 960, 1280, 1920]` | Widths for responsive image variants  |
| `image_quality`   | Integer | `85`     | Quality for image compression (1-100)                |
| `image_format`    | String  | `"webp"` | Output format for processed images                   |
| `copy_static`     | Boolean | `true`   | Copy files from `static/` to output                  |

```toml
[assets]
sass = true
minify_css = true
minify_js = true
hash_filenames = true
image_widths = [480, 800, 1200]
image_quality = 90
image_format = "avif"
```

## [taxonomies]

Define taxonomies and their behavior. Each key is a taxonomy name, and the value configures it.

| Option     | Type    | Default | Description                                  |
|------------|---------|---------|----------------------------------------------|
| `feed`     | Boolean | `false` | Generate a feed for each term                |
| `paginate` | Integer | `0`     | Number of items per page (0 = no pagination) |
| `order`    | String  | `"date"`| Sort order: `"date"`, `"title"`, `"weight"`  |
| `slug`     | String  | auto    | Override the URL path for the taxonomy       |

```toml
[taxonomies]
tags = { feed = true, paginate = 10, order = "date" }
categories = { feed = false, paginate = 20, order = "title" }
series = { feed = true, paginate = 0, order = "weight", slug = "series" }
```

Custom taxonomies are automatically supported. Just add them to frontmatter:

```yaml
---
title: "My Post"
series:
  - "Rust Fundamentals"
---
```

## [sitemap]

Sitemap generation settings.

| Option       | Type    | Default    | Description                             |
|------------- |---------|------------|-----------------------------------------|
| `enable`     | Boolean | `true`     | Generate a `sitemap.xml`                |
| `changefreq` | String  | `"weekly"` | Default change frequency for all pages  |
| `priority`   | Float   | `0.5`      | Default priority for all pages          |

```toml
[sitemap]
enable = true
changefreq = "monthly"
priority = 0.5
```

Individual pages can override these defaults in their frontmatter.

## [i18n]

Internationalization settings.

| Option           | Type   | Default  | Description                              |
|------------------|--------|----------|------------------------------------------|
| `default_locale` | String | `"en"`   | The default locale for the site          |
| `locales`        | Array  | `["en"]` | List of supported locales                |

```toml
[i18n]
default_locale = "en"
locales = ["en", "fr", "de", "es", "ja"]
```

See [Internationalization](/features/i18n/) for full details.

## [plugins]

Enable or configure plugins.

```toml
[plugins]
reading_time = true

[plugins.search]
enable = true
index_content = true

[plugins.custom]
path = "plugins/my-plugin.rhai"
```

See [Plugins](/features/plugins/) for details on available plugins and writing your own.

## [lint]

Content linting rules that check your Markdown files during every build. Warnings are informational and do not cause the build to fail.

| Option            | Type    | Default | Description                                         |
|-------------------|---------|---------|-----------------------------------------------------|
| `min_word_count`  | Integer | `0`     | Minimum word count per page (0 = disabled)          |
| `max_word_count`  | Integer | `0`     | Maximum word count per page (0 = disabled)          |
| `required_fields` | Array   | `[]`    | Frontmatter fields that every page must have        |
| `require_tags`    | Boolean | `false` | Require at least one tag per page                   |
| `require_date`    | Boolean | `false` | Require a date field on every page                  |

```toml
[lint]
min_word_count = 200
max_word_count = 8000
required_fields = ["title", "date", "description"]
require_tags = true
require_date = true
```

See [Content Linting](/features/linting/) for details on each rule and how warnings appear.

## [[remote]]

Fetch external data at build time and make it available in templates as `data.remote.<name>`. Each `[[remote]]` entry defines one data source.

| Option | Type    | Default | Description                                            |
|--------|---------|---------|--------------------------------------------------------|
| `name` | String  | --      | Key used to access the data in templates               |
| `url`  | String  | --      | URL to fetch JSON data from                            |
| `ttl`  | Integer | `3600`  | Cache duration in seconds (0 = fetch every build)      |

```toml
[[remote]]
name = "github_repos"
url = "https://api.github.com/users/myuser/repos"
ttl = 3600

[[remote]]
name = "quotes"
url = "https://api.example.com/quotes.json"
ttl = 86400
```

Access in templates:

```html
{% for repo in data.remote.github_repos %}
  <a href="{{ repo.html_url }}">{{ repo.name }}</a>
{% endfor %}
```

## Environment Variables

Some settings can be overridden with environment variables:

| Variable             | Overrides                |
|----------------------|--------------------------|
| `MYTHIC_BASE_URL`    | `site.base_url`          |
| `MYTHIC_DRAFTS`      | `build.drafts`           |
| `MYTHIC_OUTPUT`      | `build.output`           |
| `MYTHIC_PORT`        | `server.port`            |

```bash
MYTHIC_BASE_URL="https://staging.example.com" mythic build
```

Environment variables take precedence over `mythic.toml` values.
