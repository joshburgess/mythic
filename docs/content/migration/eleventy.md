---
title: Migrating from Eleventy
---
# Migrating from Eleventy

## Automatic Migration

```bash
mythic migrate --from eleventy --source /path/to/11ty-site --output my-mythic-site
```

Also accepts `--from 11ty` as an alias.

## What Gets Converted

| Eleventy | Mythic | Notes |
|----------|--------|-------|
| `.eleventy.js` | `mythic.toml` | input/data dirs extracted |
| Content (`.md`) | `content/` | Copied as-is |
| Content (`.njk`) | `content/*.html` | Nunjucks → Tera conversion |
| Content (`.liquid`) | `content/*.html` | Liquid → Tera conversion |
| `_includes/` | `templates/` | Template conversion applied |
| `_data/*.json` | `_data/` | Copied as-is |
| `_data/*.js` | — | Flagged for manual conversion |
| `dirname.json` | `_dir.yaml` | Directory data files converted |

## Nunjucks to Tera

Most Nunjucks syntax works in Tera without changes:

- `{% for %}` / `{% endfor %}` — identical
- `{% if %}` / `{% endif %}` — identical
- `{% include %}` — identical
- `{% extends %}` — identical
- `{% block %}` — identical
- `{% set %}` — identical
- `{{ variable | filter }}` — identical

Notable differences:

| Nunjucks | Tera |
|----------|------|
| `{{ data \| dump }}` | `{{ data \| json_encode() }}` |

## Manual Steps

- **JavaScript data files** (`.js` in `_data/`) cannot be auto-converted. Rewrite them as static YAML/JSON files or use Rhai plugins for dynamic data.
- **Eleventy computed data** needs reimplementation via the data cascade (`_dir.yaml`) or plugins.
- **Custom Nunjucks filters** registered in `.eleventy.js` need Tera equivalents or Rhai plugins.
- **Pagination** has no direct equivalent yet; use taxonomies for tag/category pages.
