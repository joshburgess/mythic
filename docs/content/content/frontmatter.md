---
title: "Frontmatter"
---

# Frontmatter

Every content file in Mythic can include frontmatter at the top of the file. Frontmatter defines metadata about the page such as its title, date, layout, and custom fields. Mythic supports both YAML and TOML frontmatter formats.

## YAML Frontmatter

YAML frontmatter is enclosed in triple dashes (`---`):

```markdown
---
title: "My Page Title"
date: 2026-03-21
tags:
  - rust
  - web
---

Your content starts here.
```

## TOML Frontmatter

TOML frontmatter is enclosed in triple plus signs (`+++`):

```markdown
+++
title = "My Page Title"
date = 2026-03-21
tags = ["rust", "web"]
+++

Your content starts here.
```

Both formats are fully supported. Use whichever you prefer. You can even mix formats across different files in the same project.

## Standard Fields

### title

**Type:** String
**Required:** No (but strongly recommended)

The page title, used in templates and for generating the HTML `<title>` tag.

```yaml
title: "Getting Started with Mythic"
```

### date

**Type:** Date or DateTime
**Required:** No

The publication date. Used for sorting and display. Accepts several formats:

```yaml
# Date only
date: 2026-03-21

# Date and time
date: 2026-03-21T14:30:00

# Date, time, and timezone
date: 2026-03-21T14:30:00-05:00
```

### updated

**Type:** Date or DateTime
**Required:** No

The date the content was last updated. If not set, Mythic uses the file modification time as a fallback.

```yaml
updated: 2026-03-25
```

### draft

**Type:** Boolean
**Default:** `false`

Marks the page as a draft. Draft pages are excluded from production builds but included when running `mythic serve` or `mythic build --drafts`.

```yaml
draft: true
```

### layout

**Type:** String
**Default:** Automatic (based on section or `page`)

Specifies which template to use for rendering this page. The value corresponds to a template filename without the engine extension.

```yaml
layout: blog
```

This resolves to `templates/blog.tera.html` or `templates/blog.hbs.html` depending on your engine.

### slug

**Type:** String
**Default:** Derived from filename

Overrides the URL slug. By default, the slug is the filename without the `.md` extension.

```yaml
slug: "custom-url-slug"
```

A file at `content/blog/my-post.md` with `slug: "hello-world"` renders at `/blog/hello-world/`.

### weight

**Type:** Integer
**Required:** No

Controls manual sort order within a section. Pages with lower weight appear first. Pages without a weight are sorted after weighted pages.

```yaml
weight: 10
```

### tags

**Type:** List of Strings
**Required:** No

Assigns taxonomy tags to the page:

```yaml
tags:
  - rust
  - static-sites
  - tutorial
```

### categories

**Type:** List of Strings
**Required:** No

Assigns taxonomy categories to the page:

```yaml
categories:
  - tutorials
  - web-development
```

### aliases

**Type:** List of Strings
**Required:** No

Creates redirect pages from old URLs to this page. Useful when you restructure your site or rename content. For each alias, Mythic generates an HTML file at that path containing a `<meta http-equiv="refresh">` redirect and a canonical link pointing to the current page.

```yaml
aliases:
  - /old-url/
  - /another-old-path/post-name/
```

For example, if a page lives at `/blog/new-slug/` and has the aliases above, visiting `/old-url/` will automatically redirect the browser to `/blog/new-slug/`. This is especially helpful for preserving inbound links and search engine rankings after reorganizing content.

### description

**Type:** String
**Required:** No

A short description used for meta tags and feed summaries:

```yaml
description: "A complete guide to setting up Mythic for your first project."
```

### template_engine

**Type:** String (`"tera"`, `"handlebars"`, or `"minijinja"`)
**Required:** No

Overrides the template engine for this specific page:

```yaml
template_engine: handlebars
```

## Sitemap Fields

The `sitemap` field lets you control how an individual page appears in the generated `sitemap.xml`. These per-page values override the global defaults set in `mythic.toml` under `[sitemap]`.

```yaml
sitemap:
  changefreq: weekly
  priority: 0.8
  disable: false
```

### sitemap.changefreq

How frequently the page is likely to change. Values: `always`, `hourly`, `daily`, `weekly`, `monthly`, `yearly`, `never`. If omitted, the value falls back to the global `[sitemap] changefreq` setting (default: `weekly`).

### sitemap.priority

The priority of this page relative to other pages on your site. A value between `0.0` and `1.0`. Default is `0.5`. Higher values suggest greater importance to search engine crawlers, though crawlers may interpret this differently.

### sitemap.disable

Set to `true` to exclude this page from the sitemap entirely. This is useful for utility pages, landing page variants, or any page you do not want indexed.

## Locale

For multilingual sites, specify the page locale:

```yaml
locale: fr
```

See [Internationalization](/features/i18n/) for full details on multilingual content.

## Extra Fields

The `extra` field is a free-form map for any custom data you need in templates:

```yaml
extra:
  author: "Jane Doe"
  cover_image: "/images/cover.jpg"
  reading_time: 5
  featured: true
  series:
    name: "Rust Fundamentals"
    part: 3
```

Access extra fields in templates:

```html
<!-- Tera -->
{% if page.extra.featured %}
<span class="badge">Featured</span>
{% endif %}
<img src="{{ page.extra.cover_image }}" alt="Cover">
<p>By {{ page.extra.author }}</p>
```

## Inheriting Frontmatter

Default values can be set for all pages in a section using `_dir.yaml`:

```yaml
# content/blog/_dir.yaml
layout: blog
extra:
  author: "Default Author"
  show_toc: true
```

Individual pages can override any inherited field:

```yaml
---
title: "Guest Post"
extra:
  author: "Guest Writer"
  show_toc: false
---
```

The merge is shallow by default. The page-level `extra` completely replaces the section-level `extra`. To enable deep merging:

```toml
# mythic.toml
[build]
deep_merge_frontmatter = true
```

With deep merging, only the keys specified at the page level override the defaults, and unspecified keys are preserved from the section defaults.

## Complete Example

```yaml
---
title: "Building a Blog with Mythic"
date: 2026-03-21
updated: 2026-03-25
draft: false
layout: blog
slug: "building-a-blog"
weight: 1
tags:
  - tutorial
  - mythic
categories:
  - guides
description: "Step-by-step guide to building a blog with Mythic."
aliases:
  - /tutorials/blog-setup/
locale: en
sitemap:
  changefreq: monthly
  priority: 0.9
extra:
  author: "Jane Doe"
  cover_image: "/images/blog-cover.jpg"
  featured: true
---
```
