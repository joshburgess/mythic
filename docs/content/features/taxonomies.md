---
title: "Taxonomies"
---

# Taxonomies

Taxonomies let you classify and group content. Mythic has built-in support for tags and categories and allows you to define custom taxonomies for any grouping you need.

## Configuration

Define taxonomies in `mythic.toml`:

```toml
[taxonomies]
tags = { feed = true, paginate = 10 }
categories = { feed = true, paginate = 20 }
```

Each taxonomy entry supports these options:

| Option     | Type    | Default  | Description                                   |
|------------|---------|----------|-----------------------------------------------|
| `feed`     | Boolean | `false`  | Generate an Atom feed per term                |
| `paginate` | Integer | `0`      | Items per page (0 disables pagination)        |
| `order`    | String  | `"date"` | Sort order: `"date"`, `"title"`, `"weight"`   |
| `slug`     | String  | auto     | Override the URL path                         |

## Assigning Taxonomies

Add taxonomy terms in your content frontmatter:

```yaml
---
title: "Understanding Ownership in Rust"
date: 2026-03-21
tags:
  - rust
  - memory-management
  - beginner
categories:
  - tutorials
---
```

A page can belong to multiple terms in each taxonomy.

## Generated Pages

For each taxonomy, Mythic generates:

### Taxonomy Index Page

Lists all terms in the taxonomy.

- **URL:** `/tags/` or `/categories/`
- **Template:** `taxonomy_list.tera.html`

### Term Pages

Lists all pages tagged with a specific term.

- **URL:** `/tags/rust/` or `/categories/tutorials/`
- **Template:** `taxonomy.tera.html`

### Feeds

If `feed = true`, each term gets its own Atom feed:

- `/tags/rust/atom.xml`
- `/categories/tutorials/atom.xml`

## Templates

### Taxonomy List Template

Displays all terms for a taxonomy:

```html
<!-- templates/taxonomy_list.tera.html -->
{% extends "base.tera.html" %}

{% block content %}
<h1>All {{ taxonomy.name | title }}</h1>

<ul class="taxonomy-list">
    {% for term in taxonomy.terms | sort_by(attribute="name") %}
    <li>
        <a href="/{{ taxonomy.slug }}/{{ term.slug }}/">
            {{ term.name }}
        </a>
        <span class="count">({{ term.pages | length }})</span>
    </li>
    {% endfor %}
</ul>
{% endblock %}
```

### Term Page Template

Displays pages for a single term:

```html
<!-- templates/taxonomy.tera.html -->
{% extends "base.tera.html" %}

{% block content %}
<h1>{{ term.name }}</h1>
<p>{{ term.pages | length }} posts</p>

{% for post in term.pages | sort_by(attribute="date") | reverse %}
<article class="post-summary">
    <h2><a href="{{ post.path }}">{{ post.title }}</a></h2>
    <time datetime="{{ post.date | date(format='%Y-%m-%d') }}">
        {{ post.date | date(format="%B %e, %Y") }}
    </time>
    {% if post.description %}
    <p>{{ post.description }}</p>
    {% endif %}
</article>
{% endfor %}

{% if paginator %}
<nav class="pagination">
    {% if paginator.previous_url %}
    <a href="{{ paginator.previous_url }}">Newer posts</a>
    {% endif %}
    <span>Page {{ paginator.current_page }} of {{ paginator.total_pages }}</span>
    {% if paginator.next_url %}
    <a href="{{ paginator.next_url }}">Older posts</a>
    {% endif %}
</nav>
{% endif %}
{% endblock %}
```

### Tag Cloud

Build a tag cloud by checking term page counts:

```html
<div class="tag-cloud">
{% for term in site.taxonomies.tags %}
    {% set size = term.pages | length %}
    <a href="/tags/{{ term.slug }}/"
       class="tag tag-size-{% if size > 10 %}lg{% elif size > 5 %}md{% else %}sm{% endif %}">
        {{ term.name }}
    </a>
{% endfor %}
</div>
```

## Pagination

When `paginate` is set to a value greater than 0 in the taxonomy configuration, Mythic automatically splits term pages into multiple pages. The template receives a `data.paginator` object with the following fields:

| Field                      | Type           | Description                                      |
|----------------------------|----------------|--------------------------------------------------|
| `data.paginator.pages`     | Array of Pages | The pages for the current paginated slice         |
| `data.paginator.current_page` | Integer     | The current page number (1-based)                |
| `data.paginator.total_pages`  | Integer     | Total number of paginated pages                  |
| `data.paginator.prev_url`    | String or null | URL to the previous page, null on the first page |
| `data.paginator.next_url`    | String or null | URL to the next page, null on the last page      |

Use `data.paginator.pages` instead of `term.pages` when pagination is active, since it contains only the subset of pages for the current slice:

```html
{% for post in data.paginator.pages %}
<article>
    <h2><a href="{{ post.path }}">{{ post.title }}</a></h2>
    <time>{{ post.date | date(format="%B %e, %Y") }}</time>
</article>
{% endfor %}

<nav class="pagination">
    {% if data.paginator.prev_url %}
    <a href="{{ data.paginator.prev_url }}">Previous</a>
    {% endif %}
    <span>Page {{ data.paginator.current_page }} of {{ data.paginator.total_pages }}</span>
    {% if data.paginator.next_url %}
    <a href="{{ data.paginator.next_url }}">Next</a>
    {% endif %}
</nav>
```

Paginated pages are generated at URLs like `/tags/rust/`, `/tags/rust/page/2/`, `/tags/rust/page/3/`, and so on.

## Custom Taxonomies

Define any taxonomy you need. For example, a "series" taxonomy for multi-part content:

```toml
# mythic.toml
[taxonomies]
tags = { feed = true, paginate = 10 }
categories = { feed = false, paginate = 20 }
series = { feed = true, paginate = 0, order = "weight" }
```

Use it in frontmatter:

```yaml
---
title: "Rust Fundamentals - Part 1: Variables"
series:
  - "Rust Fundamentals"
weight: 1
---
```

The series term page at `/series/rust-fundamentals/` lists all parts in weight order.

### Multiple Custom Taxonomies

```toml
[taxonomies]
tags = { feed = true, paginate = 10 }
authors = { feed = true, paginate = 10 }
topics = { feed = false, paginate = 20 }
difficulty = { feed = false, paginate = 0 }
```

```yaml
---
title: "Advanced Async Patterns"
tags: [rust, async]
authors: [jane-doe]
topics: [concurrency]
difficulty: [advanced]
---
```

## Displaying Taxonomies on Pages

Show a page's taxonomy terms in its template:

```html
<!-- In a page template -->
{% if page.tags %}
<div class="tags">
    <strong>Tags:</strong>
    {% for tag in page.tags %}
    <a href="/tags/{{ tag | slugify }}/" class="tag">{{ tag }}</a>
    {% if not loop.last %}, {% endif %}
    {% endfor %}
</div>
{% endif %}

{% if page.categories %}
<div class="categories">
    <strong>Category:</strong>
    {% for cat in page.categories %}
    <a href="/categories/{{ cat | slugify }}/">{{ cat }}</a>
    {% endfor %}
</div>
{% endif %}
```

## URL Customization

Override the URL slug for a taxonomy:

```toml
[taxonomies]
tags = { slug = "topics" }        # /topics/ instead of /tags/
categories = { slug = "sections" } # /sections/ instead of /categories/
```
