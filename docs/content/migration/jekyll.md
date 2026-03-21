---
title: Migrating from Jekyll
---
# Migrating from Jekyll

## Automatic Migration

```bash
mythic migrate --from jekyll --source /path/to/jekyll-site --output my-mythic-site
```

## What Gets Converted

| Jekyll | Mythic | Notes |
|--------|--------|-------|
| `_config.yml` | `mythic.toml` | title, url, description mapped |
| `_posts/YYYY-MM-DD-title.md` | `content/posts/title.md` | Date extracted to frontmatter |
| `_layouts/` | `templates/` | Liquid → Tera syntax conversion |
| `_includes/` | `templates/partials/` | Copied as-is |
| `_data/` | `_data/` | Copied as-is |

## Template Syntax Changes

| Jekyll (Liquid) | Mythic (Tera) |
|-----------------|---------------|
| `{{ content }}` | `{{ content \| safe }}` |
| `{% elsif %}` | `{% elif %}` |
| `{% assign x = 1 %}` | `{% set x = 1 %}` |
| `{% include file.html %}` | `{% include "file.html" %}` |

## Manual Steps

After migration, review the report for warnings. Common items needing manual attention:

- Liquid filters (`| date:`, `| where:`, `| sort:`) need Tera equivalents
- `{% capture %}` blocks need conversion to `{% set %}`
- Jekyll plugins have no direct equivalent (use Mythic's plugin system or Rhai scripts)
- Gem-based themes need their layouts extracted manually
