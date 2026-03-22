---
title: "Migrating from Hugo"
---

# Migrating from Hugo

Mythic includes a migration tool that converts Hugo sites to Mythic's structure. It handles content, configuration, data files, and performs basic Go template-to-Tera syntax conversion.

## Running the Migration

```bash
mythic migrate --from hugo --source /path/to/hugo-site --output my-mythic-site
```

The tool automatically detects Hugo's configuration file, whether it is `config.toml`, `hugo.toml`, `config.yaml`, or `hugo.yaml`.

### Options

| Flag         | Default | Description                                    |
|--------------|---------|------------------------------------------------|
| `--source`   | `.`     | Path to the Hugo project                       |
| `--output`   | `./out` | Path for the new Mythic project                |
| `--dry-run`  | `false` | Preview changes without writing files          |
| `--verbose`  | `false` | Show detailed conversion output                |

## What Gets Converted

### Content Files

| Hugo                            | Mythic                              |
|---------------------------------|-------------------------------------|
| `content/posts/my-post.md`     | `content/posts/my-post.md`          |
| `content/about.md`             | `content/about.md`                  |
| `content/_index.md`            | `content/index.md`                  |
| `content/posts/_index.md`     | `content/posts/index.md`           |

Hugo's content directory structure is preserved. `_index.md` files are renamed to `index.md`.

### Frontmatter

Hugo frontmatter is converted to Mythic's format:

```yaml
# Hugo
---
title: "My Post"
date: 2026-03-21T14:30:00-05:00
draft: false
tags: ["rust", "web"]
categories: ["tutorials"]
weight: 10
summary: "A quick introduction to Rust."
params:
  author: "Jane Doe"
  featured: true
---

# Mythic (converted)
---
title: "My Post"
date: 2026-03-21T14:30:00-05:00
draft: false
tags:
  - rust
  - web
categories:
  - tutorials
weight: 10
description: "A quick introduction to Rust."
extra:
  author: "Jane Doe"
  featured: true
---
```

Key conversions:

- `summary` becomes `description`
- `params` (or `Params`) becomes `extra`
- `slug`, `url` are converted to Mythic's `slug` field
- `aliases` is preserved as-is (same format)

### Configuration

Hugo's config is converted to `mythic.toml`:

```toml
# Hugo config.toml
baseURL = "https://example.com/"
languageCode = "en-us"
title = "My Hugo Site"
theme = "my-theme"

[params]
  description = "A Hugo site"
  author = "Jane Doe"

[taxonomies]
  tag = "tags"
  category = "categories"
  series = "series"

[markup]
  [markup.highlight]
    style = "monokai"
```

```toml
# Mythic mythic.toml (converted)
[site]
title = "My Hugo Site"
base_url = "https://example.com"
language = "en"
description = "A Hugo site"

[taxonomies]
tags = { feed = true }
categories = { feed = true }
series = { feed = true }

[markdown]
syntax_theme = "monokai"

[build]
template_engine = "tera"
```

### Data Files

| Hugo               | Mythic              |
|--------------------|---------------------|
| `data/nav.toml`    | `_data/nav.toml`    |
| `data/authors.yaml`| `_data/authors.yaml`|
| `data/team.json`   | `_data/team.json`   |

Data files are copied directly. Access patterns change:

```
# Hugo:    {{ .Site.Data.nav }}
# Mythic:  {{ data.nav }}
```

### Static Files

Hugo's `static/` directory is copied to Mythic's `static/` as-is.

### Shortcodes

Hugo shortcodes in `layouts/shortcodes/` are converted to `templates/shortcodes/`:

```html
<!-- Hugo: layouts/shortcodes/youtube.html -->
<div class="video">
    <iframe src="https://www.youtube.com/embed/{{ .Get "id" }}" allowfullscreen></iframe>
</div>

<!-- Mythic: templates/shortcodes/youtube.tera.html (converted) -->
<div class="video">
    <iframe src="https://www.youtube.com/embed/{{ id }}" allowfullscreen></iframe>
</div>
```

Shortcode syntax conversions:

| Hugo Shortcode                | Mythic Shortcode              |
|-------------------------------|-------------------------------|
| `{{ .Get "param" }}`         | `{{ param }}`                 |
| `{{ .Get 0 }}`               | `{{ _args[0] }}`             |
| `{{ .Inner }}`               | `{{ body \| safe }}`         |
| `{{ .InnerDeindent }}`       | `{{ body \| safe }}`         |
| `{{ .Page.Title }}`          | `{{ page.title }}`            |
| `{{ .Site.Title }}`          | `{{ site.title }}`            |

Content shortcode invocations are also converted:

```
# Hugo
{{< youtube id="dQw4w9WgXcQ" >}}
{{% callout %}}content{{% /callout %}}

# Mythic (converted)
{{ youtube(id="dQw4w9WgXcQ") }}
{% callout() %}content{% end %}
```

## Go Template Conversions

The migration tool performs automatic syntax conversion for Go templates. Here is the full reference:

### Variables and Output

| Hugo (Go Templates)              | Mythic (Tera)                              |
|----------------------------------|---------------------------------------------|
| `{{ .Title }}`                   | `{{ page.title }}`                          |
| `{{ .Content }}`                 | `{{ content \| safe }}`                     |
| `{{ .Summary }}`                 | `{{ page.description }}`                    |
| `{{ .Date }}`                    | `{{ page.date }}`                           |
| `{{ .Params.author }}`          | `{{ page.extra.author }}`                   |
| `{{ .Site.Title }}`              | `{{ site.title }}`                          |
| `{{ .Site.BaseURL }}`           | `{{ site.base_url }}`                       |
| `{{ .Site.Data.nav }}`          | `{{ data.nav }}`                            |
| `{{ .Permalink }}`              | `{{ site.base_url }}{{ page.path }}`        |
| `{{ .RelPermalink }}`           | `{{ page.path }}`                           |
| `{{ .WordCount }}`              | `{{ page.word_count }}`                     |
| `{{ .ReadingTime }}`            | `{{ page.reading_time }}`                   |

### Control Structures

| Hugo                                      | Mythic                                          |
|-------------------------------------------|-------------------------------------------------|
| `{{ range .Pages }}`                     | `{% for page in pages %}`                       |
| `{{ end }}`                              | `{% endfor %}` or `{% endif %}`                 |
| `{{ if .Params.featured }}`             | `{% if page.extra.featured %}`                  |
| `{{ else if .Draft }}`                   | `{% elif page.draft %}`                         |
| `{{ else }}`                             | `{% else %}`                                    |
| `{{ with .Params.author }}`             | `{% if page.extra.author %}`                    |
| `{{ partial "header.html" . }}`         | `{% include "partials/header.tera.html" %}`     |
| `{{ block "main" . }}`                  | `{% block main %}`                              |
| `{{ define "main" }}`                   | `{% block main %}`                              |

### Functions and Pipes

| Hugo                                       | Mythic                                          |
|--------------------------------------------|-------------------------------------------------|
| `{{ .Date \| time.Format "2006-01-02" }}` | `{{ page.date \| date(format="%Y-%m-%d") }}`   |
| `{{ safeHTML .Content }}`                  | `{{ content \| safe }}`                         |
| `{{ truncate 100 .Summary }}`             | `{{ page.description \| truncate(length=100) }}`|
| `{{ upper .Title }}`                       | `{{ page.title \| upper }}`                     |
| `{{ lower .Title }}`                       | `{{ page.title \| lower }}`                     |
| `{{ len .Pages }}`                         | `{{ pages \| length }}`                         |
| `{{ sort .Pages "Date" "desc" }}`         | `{{ pages \| sort_by(attribute="date") \| reverse }}` |

## What Requires Manual Conversion

### Template with Blocks

Hugo's `with` blocks change the context dot (`.`), which has no direct equivalent in Tera:

```go
<!-- Hugo -->
{{ with .Params.author }}
  <span>By {{ . }}</span>
{{ end }}

<!-- Tera (manual conversion) -->
{% if page.extra.author %}
  <span>By {{ page.extra.author }}</span>
{% endif %}
```

### Hugo Pipes (Asset Pipeline)

Hugo's asset pipeline functions need to be replaced with Mythic's asset system:

```go
<!-- Hugo -->
{{ $style := resources.Get "css/main.scss" | toCSS | minify | fingerprint }}
<link rel="stylesheet" href="{{ $style.RelPermalink }}">

<!-- Mythic -->
<link rel="stylesheet" href="{{ assets.css }}">
```

### Hugo Modules and Themes

Hugo modules and theme components cannot be automatically extracted. If your Hugo site uses a theme:

1. Copy the theme's `layouts/` directory into your Hugo site before migrating
2. Merge theme assets with site assets
3. Then run the migration

### Complex Go Template Logic

Some Hugo-specific functions have no direct equivalent:

- `dict` and `slice` (use Tera's native syntax)
- `$.Scratch` (use `{% set %}` variables)
- `.Page.Resources` (use Mythic's asset pipeline)
- `markdownify` (content is already processed as Markdown)
- Custom output formats (manual configuration needed)

## Step-by-Step Migration

1. **Extract theme layouts** if using a Hugo theme:
   ```bash
   cp -r themes/my-theme/layouts/* layouts/
   cp -r themes/my-theme/static/* static/
   ```

2. **Run the migration:**
   ```bash
   mythic migrate --from hugo --source . --output ../mythic-site --verbose
   ```

3. **Review the migration report** for warnings about unconverted templates.

4. **Manually fix templates** in `templates/`, focusing on `with` blocks and Hugo pipe functions.

5. **Build and test:**
   ```bash
   cd ../mythic-site
   mythic build
   mythic serve
   ```

6. **Compare pages** side by side with your Hugo site to catch visual differences.

---

## Converting Hugo Themes

Mythic can convert Hugo themes into Mythic starter templates, giving you access to Hugo's 400+ theme ecosystem:

```bash
mythic migrate --from hugo-theme --source /path/to/hugo-theme --output my-theme
```

### What Gets Converted

| Hugo Theme | Mythic Starter | Notes |
|---|---|---|
| `layouts/_default/baseof.html` | `templates/base.html` | Tera `{% extends %}` pattern |
| `layouts/_default/single.html` | `templates/default.html` | Main page template |
| `layouts/_default/list.html` | `templates/list.html` | Listing pages |
| `layouts/partials/*` | `templates/partials/*` | Included partials |
| `layouts/shortcodes/*` | `shortcodes/*` | Shortcode templates |
| `assets/css/*` | `styles/*` | CSS/SCSS files |
| `assets/js/*` | `scripts/*` | JavaScript files |
| `static/*` | `static/*` | Static assets |
| `archetypes/*` | `content/*.md` | Example content |
| `exampleSite/content/*` | `content/*` | Demo content |
| `theme.toml` | `mythic.toml` | Site config |

### Template Syntax Conversion

These patterns are converted automatically:

| Hugo (Go Templates) | Mythic (Tera) |
|---|---|
| `{{ .Title }}` | `{{ page.title }}` |
| `{{ .Content }}` | `{{ content \| safe }}` |
| `{{ .Params.author }}` | `{{ page.extra.author }}` |
| `{{ .Site.Title }}` | `{{ site.title }}` |
| `{{ .RelPermalink }}` | `{{ page.url }}` |
| `{{ .WordCount }}` | `{{ content \| word_count }}` |
| `{{ .ReadingTime }}` | `{{ content \| reading_time }}` |
| `{{ partial "header.html" . }}` | `{% include "header.html" %}` |
| `{{ range .Pages }}` | `{% for item in pages %}` |
| `{{ end }}` | `{% endfor %}` / `{% endif %}` |
| `{{ with .Params.X }}` | `{% if page.extra.X %}` |
| `{{ block "main" . }}` | `{% block main %}` |
| `{{ define "main" }}` | `{% block main %}` |
| `{{ else if }}` | `{% elif %}` |
| `\| safeHTML` | `\| safe` |

### Hugo Filters That Work at Runtime

These Hugo filters are registered as Tera filters — converted templates using them work without modification:

| Hugo Filter | Mythic Equivalent | What It Does |
|---|---|---|
| `\| markdownify` | `\| markdownify` | Renders markdown to HTML inline |
| `\| plainify` | `\| plainify` | Strips HTML tags to plain text |
| `\| humanize` | `\| humanize` | `"my-slug"` → `"My Slug"` |
| `\| pluralize` | `\| pluralize` | `"post"` → `"posts"` |
| `\| singularize` | `\| singularize` | `"posts"` → `"post"` |
| `\| urlize` | `\| urlize` | `"My Title"` → `"my-title"` |
| `\| safeHTML` | `\| safeHTML` | Marks content as safe |

### What Requires Manual Conversion

**Hugo Pipes (asset processing in templates):**

Hugo processes assets inline in templates. Mythic handles this in the build pipeline.

```go
{{/* Hugo */}}
{{ $style := resources.Get "css/main.scss" | toCSS | minify | fingerprint }}
<link rel="stylesheet" href="{{ $style.RelPermalink }}" integrity="{{ $style.Data.Integrity }}">
```

Replace with Mythic's asset pipeline:

```html
{# Mythic #}
<link rel="stylesheet" href="{{ assets.css_path }}" integrity="{{ assets.css_integrity }}" crossorigin="anonymous">
```

**Go data constructors (`dict` / `slice`):**

```go
{{/* Hugo */}}
{{ partial "header.html" (dict "title" .Title "show_nav" true) }}
```

Replace with Tera includes or template variables:

```html
{# Mythic #}
{% set header_title = page.title %}
{% include "partials/header.html" %}
```

### Conversion Report

The converter generates a `CONVERSION.md` file in the output directory listing:
- Total files converted and copied
- Warnings for patterns that need manual attention
- Step-by-step instructions for finishing the conversion

### Real-World Results

Tested against popular Hugo themes:

| Theme | Templates | Auto-Converted | Manual Fixes Needed |
|---|---:|---:|---|
| PaperMod | 40 | ~80% | Hugo Pipes, dict/slice |
| Ananke | 38 | ~85% | Hugo Pipes, dict/slice |
| Congo | 61 | ~75% | Hugo Pipes, Scratch, dict/slice |

Most manual fixes involve replacing Hugo Pipes with Mythic's `{{ assets.css_path }}` — typically 5-10 lines per theme.
