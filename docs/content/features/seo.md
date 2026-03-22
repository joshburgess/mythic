---
title: "SEO"
---

# SEO

Mythic provides built-in SEO features that help your site rank well in search engines without requiring external plugins or manual markup. These features include structured data, subresource integrity, sitemaps, and robots.txt generation.

## Schema.org JSON-LD

Mythic automatically generates [Schema.org](https://schema.org/) structured data as JSON-LD and injects it into your pages. This helps search engines understand the type and structure of your content.

### Supported Schema Types

Mythic selects the appropriate schema type based on the page's location and frontmatter:

| Schema Type       | When it is used                                                    |
|-------------------|--------------------------------------------------------------------|
| `BlogPosting`     | Pages inside a `blog/` section with a `date` field                 |
| `Article`         | Pages with a `date` field in other sections                        |
| `WebPage`         | Pages without a `date` field (e.g., about, contact)                |
| `BreadcrumbList`  | Automatically added to all pages based on URL hierarchy            |

### Generated Output

For a blog post with the following frontmatter:

```yaml
---
title: "Understanding Rust Lifetimes"
date: 2026-03-15
description: "A practical guide to lifetime annotations in Rust."
extra:
  author: "Jane Doe"
---
```

Mythic generates:

```html
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "BlogPosting",
  "headline": "Understanding Rust Lifetimes",
  "datePublished": "2026-03-15",
  "description": "A practical guide to lifetime annotations in Rust.",
  "author": {
    "@type": "Person",
    "name": "Jane Doe"
  },
  "mainEntityOfPage": {
    "@type": "WebPage",
    "@id": "https://example.com/blog/understanding-rust-lifetimes/"
  }
}
</script>
```

A `BreadcrumbList` is also generated for every page:

```html
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  "itemListElement": [
    { "@type": "ListItem", "position": 1, "name": "Home", "item": "https://example.com/" },
    { "@type": "ListItem", "position": 2, "name": "Blog", "item": "https://example.com/blog/" },
    { "@type": "ListItem", "position": 3, "name": "Understanding Rust Lifetimes" }
  ]
}
</script>
```

### Adding Author and Description

The `author` and `description` fields used in JSON-LD come from your page frontmatter. Use the `extra` block for author information:

```yaml
---
title: "My Post"
date: 2026-03-15
description: "A short summary for search engines and social cards."
extra:
  author: "Jane Doe"
---
```

If no `extra.author` is set, Mythic falls back to the site-level author if one is defined in `_data/site.yaml` or similar data files.

The `description` field is also used for the `<meta name="description">` tag.

## Subresource Integrity (SRI)

Mythic generates [Subresource Integrity](https://developer.mozilla.org/en-US/docs/Web/Security/Subresource_Integrity) hashes for your CSS and JavaScript files. SRI allows browsers to verify that fetched resources have not been tampered with.

Two template variables are available:

| Variable                  | Description                                 |
|---------------------------|---------------------------------------------|
| `{{ assets.css_integrity }}` | SHA-384 hash of the compiled CSS file    |
| `{{ assets.js_integrity }}`  | SHA-384 hash of the bundled JS file      |

### Example Template with SRI

```html
<link rel="stylesheet"
      href="{{ assets.css }}"
      integrity="{{ assets.css_integrity }}"
      crossorigin="anonymous">

<script src="{{ assets.js }}"
        integrity="{{ assets.js_integrity }}"
        crossorigin="anonymous"
        defer></script>
```

This renders to something like:

```html
<link rel="stylesheet"
      href="/css/main.a1b2c3d4.css"
      integrity="sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8w"
      crossorigin="anonymous">
```

The `crossorigin="anonymous"` attribute is required when using integrity hashes. SRI hashes are recalculated on every build, so they always reflect the current file contents.

## Sitemap and robots.txt

Mythic generates a `sitemap.xml` and a `robots.txt` automatically. See the [Configuration Reference](/configuration/reference/) for sitemap options.

The sitemap includes all non-draft pages with their last-modified dates and change frequencies. The `robots.txt` file points search engine crawlers to the sitemap:

```
User-agent: *
Allow: /
Sitemap: https://example.com/sitemap.xml
```

Both files are placed in the root of the output directory. Configure sitemap behavior in `mythic.toml`:

```toml
[sitemap]
enable = true
changefreq = "weekly"
priority = 0.5
```

Individual pages can override the default change frequency and priority in their frontmatter:

```yaml
---
title: "Home"
sitemap:
  changefreq: "daily"
  priority: 1.0
---
```
