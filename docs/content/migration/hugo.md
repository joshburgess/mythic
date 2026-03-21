---
title: Migrating from Hugo
---
# Migrating from Hugo

## Automatic Migration

```bash
mythic migrate --from hugo --source /path/to/hugo-site --output my-mythic-site
```

Supports `config.toml`, `hugo.toml`, and `config.yaml` config files.

## What Gets Converted

| Hugo | Mythic | Notes |
|------|--------|-------|
| `config.toml` / `hugo.toml` | `mythic.toml` | title, baseURL mapped |
| `content/` | `content/` | Directory structure preserved |
| `layouts/` | `templates/` | Go template → Tera conversion |
| `layouts/shortcodes/` | `shortcodes/` | `.Get` and `.Inner` converted |
| `static/` | `static/` | Copied as-is |
| `data/` | `_data/` | Copied as-is |

## Template Syntax Changes

| Hugo (Go Templates) | Mythic (Tera) |
|---------------------|---------------|
| `{{ .Title }}` | `{{ page.title }}` |
| `{{ .Content }}` | `{{ content \| safe }}` |
| `{{ .Params.x }}` | `{{ page.extra.x }}` |
| `{{ .Site.Title }}` | `{{ site.title }}` |
| `{{ partial "x.html" . }}` | `{% include "x.html" %}` |
| `{{ range .Pages }}` | `{% for item in pages %}` |
| `{{ end }}` | `{% endfor %}` / `{% endif %}` |

## Shortcode Conversion

Hugo shortcodes are automatically converted:

| Hugo | Mythic |
|------|--------|
| `{{ .Get "param" }}` | `{{ param }}` |
| `{{ .Inner }}` | `{{ inner }}` |

## Manual Steps

- `{{ with }}` blocks need manual conversion
- `{{ block }}` / `{{ define }}` → Tera's `{% block %}` / `{% extends %}`
- Hugo's `safeHTML` → Tera's `| safe`
- Hugo modules and theme components need manual extraction
