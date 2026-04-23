---
title: "Data Files"
---

# Data Files

Mythic's data file system lets you define structured data outside of your content and access it in any template. This is useful for navigation menus, author lists, site settings, and any data that does not belong to a single page.

## The _data Directory

Place data files in the `_data/` directory at the root of your project. Mythic loads all files at build time and makes them available in templates under the `data` variable.

```
_data/
  site.yaml
  authors.json
  nav.toml
```

These are accessible in templates as:

```html
{{ data.site.title }}
{{ data.authors[0].name }}
{{ data.nav.main[0].url }}
```

## Supported Formats

Mythic supports three data file formats:

### YAML

```yaml
# _data/site.yaml
title: "My Site"
tagline: "Built with Mythic"
social:
  twitter: "@mysite"
  github: "mysite"
```

### TOML

```toml
# _data/nav.toml
[[main]]
label = "Home"
url = "/"

[[main]]
label = "Blog"
url = "/blog/"

[[main]]
label = "About"
url = "/about/"
```

### JSON

```json
// _data/authors.json
[
  {
    "id": "jane",
    "name": "Jane Doe",
    "bio": "Rust developer and technical writer.",
    "avatar": "/images/jane.jpg"
  },
  {
    "id": "john",
    "name": "John Smith",
    "bio": "Frontend engineer and designer.",
    "avatar": "/images/john.jpg"
  }
]
```

## Nested Namespaces

Subdirectories inside `_data/` create nested namespaces:

```
_data/
  social/
    links.yaml
    icons.yaml
  products/
    featured.json
    categories.toml
```

Access nested data with dot notation:

```html
{% for link in data.social.links %}
  <a href="{{ link.url }}">{{ link.name }}</a>
{% endfor %}

{% for product in data.products.featured %}
  <div class="product">{{ product.name }}</div>
{% endfor %}
```

You can nest as deeply as needed:

```
_data/
  company/
    departments/
      engineering.yaml    -> data.company.departments.engineering
      marketing.yaml      -> data.company.departments.marketing
```

## Data Cascade with _dir.yaml

The `_dir.yaml` file is a special data file that can be placed inside any content directory. It defines default frontmatter values for all pages in that directory and its subdirectories.

```yaml
# content/blog/_dir.yaml
layout: blog
extra:
  sidebar: true
  author: "Editorial Team"
```

### Cascade Rules

Data cascades downward through the directory tree. A `_dir.yaml` at a deeper level overrides values from parent directories:

```
content/
  _dir.yaml                  # Global defaults
  blog/
    _dir.yaml                # Blog section defaults (overrides global)
    tutorials/
      _dir.yaml              # Tutorial defaults (overrides blog)
      my-tutorial.md          # Inherits from all three
```

Example:

```yaml
# content/_dir.yaml
extra:
  show_sidebar: true
  theme: "light"

# content/blog/_dir.yaml
layout: blog
extra:
  show_sidebar: true
  theme: "dark"

# content/blog/tutorials/_dir.yaml
layout: tutorial
extra:
  show_sidebar: false
```

A page at `content/blog/tutorials/my-tutorial.md` receives:

- `layout: tutorial` (from tutorials `_dir.yaml`)
- `extra.show_sidebar: false` (from tutorials `_dir.yaml`)
- `extra.theme: "dark"` (from blog `_dir.yaml`)

### Supported Formats

The directory data file can use any supported format:

- `_dir.yaml` or `_dir.yml`
- `_dir.toml`
- `_dir.json`

If multiple formats exist in the same directory, Mythic uses YAML first, then TOML, then JSON.

## Using Data in Templates

### Tera

```html
<!-- Iterate over a list -->
<nav>
  {% for item in data.nav.main %}
    <a href="{{ item.url }}"
       {% if item.url == page.url %}class="active"{% endif %}>
      {{ item.label }}
    </a>
  {% endfor %}
</nav>

<!-- Access nested values -->
<footer>
  <a href="https://twitter.com/{{ data.site.social.twitter }}">Twitter</a>
</footer>

<!-- Conditional checks -->
{% if data.site.announcement %}
<div class="banner">{{ data.site.announcement }}</div>
{% endif %}
```

### Handlebars

```html
<!-- Iterate over a list -->
<nav>
  {{#each data.nav.main}}
    <a href="{{this.url}}">{{this.label}}</a>
  {{/each}}
</nav>

<!-- Access nested values -->
<footer>
  <a href="https://twitter.com/{{data.site.social.twitter}}">Twitter</a>
</footer>
```

## Referencing Data in Frontmatter

You cannot directly reference data files in frontmatter. However, you can use data files to define values that templates look up:

```yaml
# content/blog/my-post.md
---
title: "My Post"
extra:
  author_id: jane
---
```

```html
<!-- templates/blog.tera.html -->
{% set author = false %}
{% for a in data.authors %}
  {% if a.id == page.extra.author_id %}
    {% set author = a %}
  {% endif %}
{% endfor %}

{% if author %}
<div class="author">
  <img src="{{ author.avatar }}" alt="{{ author.name }}">
  <p>{{ author.name }}</p>
  <p>{{ author.bio }}</p>
</div>
{% endif %}
```

## Performance

Data files are loaded once at build startup and cached in memory. They do not trigger full rebuilds during development; only templates that reference the changed data file are re-rendered.

Very large data files (hundreds of megabytes) may increase memory usage. If you need to work with large datasets, consider pre-processing them into smaller, focused files.
