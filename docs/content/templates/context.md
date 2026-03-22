---
title: "Template Context"
---

# Template Context

Every template in Mythic receives a context object containing page metadata, site information, rendered content, and data files. This reference documents all available variables.

## page

The `page` object contains metadata about the current page being rendered.

| Variable               | Type          | Description                                      |
|------------------------|---------------|--------------------------------------------------|
| `page.title`           | String        | Page title from frontmatter                      |
| `page.date`            | DateTime      | Publication date                                 |
| `page.updated`         | DateTime      | Last updated date                                |
| `page.draft`           | Boolean       | Whether the page is a draft                      |
| `page.slug`            | String        | URL slug                                         |
| `page.path`            | String        | Full URL path (e.g., `/blog/my-post/`)           |
| `page.layout`          | String        | Template name used for rendering                 |
| `page.description`     | String        | Page description from frontmatter                |
| `page.tags`            | Array         | List of tag strings                              |
| `page.categories`      | Array         | List of category strings                         |
| `page.weight`          | Integer       | Sort weight                                      |
| `page.locale`          | String        | Page locale (e.g., `en`, `fr`)                   |
| `page.word_count`      | Integer       | Number of words in the content                   |
| `page.reading_time`    | Integer       | Estimated reading time in minutes                |
| `page.file`            | String        | Source file path relative to content/             |
| `page.dir`             | String        | Directory containing the source file             |
| `page.extra`           | Object        | Custom fields from frontmatter `extra`           |
| `page.aliases`         | Array         | List of redirect aliases                         |

### Usage Examples

```html
<!-- Tera -->
<h1>{{ page.title }}</h1>

{% if page.date %}
<time datetime="{{ page.date | date(format='%Y-%m-%d') }}">
    {{ page.date | date(format="%B %e, %Y") }}
</time>
{% endif %}

{% if page.tags %}
<ul class="tags">
    {% for tag in page.tags %}
    <li><a href="/tags/{{ tag | slugify }}/">{{ tag }}</a></li>
    {% endfor %}
</ul>
{% endif %}

<p>{{ page.word_count }} words &middot; {{ page.reading_time }} min read</p>
```

```html
<!-- Handlebars -->
<h1>{{page.title}}</h1>

{{#if page.date}}
<time datetime="{{date_format page.date "%Y-%m-%d"}}">
    {{date_format page.date "%B %e, %Y"}}
</time>
{{/if}}

{{#if page.tags}}
<ul class="tags">
    {{#each page.tags}}
    <li><a href="/tags/{{slugify this}}/">{{this}}</a></li>
    {{/each}}
</ul>
{{/if}}
```

## content

The rendered HTML content of the current page. This is the Markdown body converted to HTML, with shortcodes processed.

```html
<!-- Tera: must use safe filter -->
<div class="content">
    {{ content | safe }}
</div>

<!-- Handlebars: must use triple-stash -->
<div class="content">
    {{{content}}}
</div>
```

Always use `safe` (Tera) or triple curly braces (Handlebars) to prevent HTML escaping.

## toc

The generated table of contents as an HTML list, built from the headings in the content.

```html
<!-- Tera -->
{% if toc %}
<nav class="table-of-contents">
    <h2>Contents</h2>
    {{ toc | safe }}
</nav>
{% endif %}

<!-- Handlebars -->
{{#if toc}}
<nav class="table-of-contents">
    <h2>Contents</h2>
    {{{toc}}}
</nav>
{{/if}}
```

The TOC is an ordered nested list of `<ul>` and `<li>` elements with anchor links.

## site

The `site` object contains global site information and page collections.

| Variable              | Type     | Description                                        |
|-----------------------|----------|----------------------------------------------------|
| `site.title`          | String   | Site title from `mythic.toml`                      |
| `site.base_url`       | String   | Base URL (e.g., `https://example.com`)             |
| `site.language`       | String   | Default language code                              |
| `site.description`    | String   | Site description                                   |
| `site.pages`          | Array    | All non-draft pages                                |
| `site.sections`       | Object   | Pages grouped by content section                   |
| `site.taxonomies`     | Object   | Taxonomy terms and their pages                     |
| `site.build_time`     | DateTime | Timestamp of the current build                     |

### site.pages

An array of all non-draft pages in the site. Each element has the same fields as `page`:

```html
{% for post in site.pages | sort_by(attribute="date") | reverse %}
    {% if post.dir == "blog" %}
    <article>
        <h2><a href="{{ post.path }}">{{ post.title }}</a></h2>
        <time>{{ post.date | date(format="%Y-%m-%d") }}</time>
        <p>{{ post.description }}</p>
    </article>
    {% endif %}
{% endfor %}
```

### site.sections

Pages grouped by their content directory:

```html
<!-- List all blog posts -->
{% for post in site.sections.blog | sort_by(attribute="date") | reverse %}
    <a href="{{ post.path }}">{{ post.title }}</a>
{% endfor %}

<!-- List documentation pages by weight -->
{% for doc in site.sections.docs | sort_by(attribute="weight") %}
    <a href="{{ doc.path }}">{{ doc.title }}</a>
{% endfor %}
```

### site.taxonomies

Access taxonomy terms and their associated pages:

```html
<!-- List all tags -->
{% for tag in site.taxonomies.tags %}
    <a href="/tags/{{ tag.slug }}/">
        {{ tag.name }} ({{ tag.pages | length }})
    </a>
{% endfor %}

<!-- In a taxonomy template, list pages for the current term -->
{% for post in taxonomy_pages %}
    <a href="{{ post.path }}">{{ post.title }}</a>
{% endfor %}
```

## assets

The `assets` object provides paths to processed asset files, including content hashes for cache busting.

| Variable          | Type   | Description                                |
|-------------------|--------|--------------------------------------------|
| `assets.css`      | String | Path to the compiled CSS file              |
| `assets.js`       | String | Path to the bundled JavaScript file        |

```html
<link rel="stylesheet" href="{{ assets.css }}">
<script src="{{ assets.js }}" defer></script>
```

In production, these paths include content hashes:

```html
<link rel="stylesheet" href="/css/main.a1b2c3d4.css">
<script src="/js/main.e5f6a7b8.js" defer></script>
```

## data

The `data` object contains all loaded data files from the `_data/` directory. Keys correspond to filenames without extensions, and subdirectories create nested objects.

```
_data/
  site.yaml       -> data.site
  nav.toml        -> data.nav
  authors.json    -> data.authors
  social/
    links.yaml    -> data.social.links
```

```html
<!-- Access data -->
<p>{{ data.site.tagline }}</p>

{% for item in data.nav.main %}
    <a href="{{ item.url }}">{{ item.label }}</a>
{% endfor %}

{% for author in data.authors %}
    <span>{{ author.name }}</span>
{% endfor %}
```

## Taxonomy Template Context

When rendering taxonomy listing pages, additional variables are available:

| Variable          | Type   | Description                                      |
|-------------------|--------|--------------------------------------------------|
| `taxonomy.name`   | String | Taxonomy name (e.g., `tags`)                     |
| `taxonomy.slug`   | String | URL slug of the taxonomy                         |
| `term.name`       | String | Current term name (e.g., `rust`)                 |
| `term.slug`       | String | URL slug of the term                             |
| `term.pages`      | Array  | Pages associated with this term                  |

```html
<!-- templates/taxonomy.tera.html -->
<h1>{{ term.name }}</h1>
<p>{{ term.pages | length }} posts tagged "{{ term.name }}"</p>

{% for post in term.pages | sort_by(attribute="date") | reverse %}
<article>
    <h2><a href="{{ post.path }}">{{ post.title }}</a></h2>
    <time>{{ post.date | date(format="%Y-%m-%d") }}</time>
</article>
{% endfor %}
```

## Pagination Context

When pagination is enabled, a `paginator` object is available:

| Variable                  | Type    | Description                          |
|---------------------------|---------|--------------------------------------|
| `paginator.pages`         | Array   | Pages for the current page number    |
| `paginator.current_page`  | Integer | Current page number (1-based)        |
| `paginator.total_pages`   | Integer | Total number of pages                |
| `paginator.previous_url`  | String  | URL to the previous page (or null)   |
| `paginator.next_url`      | String  | URL to the next page (or null)       |
| `paginator.total_items`   | Integer | Total number of items                |

```html
{% for post in paginator.pages %}
<article>
    <h2>{{ post.title }}</h2>
</article>
{% endfor %}

<nav class="pagination">
    {% if paginator.previous_url %}
    <a href="{{ paginator.previous_url }}">Previous</a>
    {% endif %}

    <span>Page {{ paginator.current_page }} of {{ paginator.total_pages }}</span>

    {% if paginator.next_url %}
    <a href="{{ paginator.next_url }}">Next</a>
    {% endif %}
</nav>
```

## Content Collections

All pages and section groupings are available via `data`:

### `data.pages`

Array of all non-draft pages with title, slug, url, date, and tags:

```html
<ul>
{% for p in data.pages %}
  <li><a href="{{ p.url }}">{{ p.title }}</a> — {{ p.date }}</li>
{% endfor %}
</ul>
```

### `data.sections`

Pages grouped by their top-level directory. For example, pages in `content/blog/` are available as `data.sections.blog`:

```html
<h2>Blog Posts</h2>
{% for post in data.sections.blog %}
  <article>
    <a href="{{ post.url }}">{{ post.title }}</a>
  </article>
{% endfor %}
```

## SRI Integrity Hashes

The `assets` object also provides Subresource Integrity hashes for your CSS and JavaScript files, allowing browsers to verify that fetched resources have not been tampered with.

| Variable                     | Type   | Description                                    |
|------------------------------|--------|------------------------------------------------|
| `assets.css_integrity`       | String | SHA-384 integrity hash of the compiled CSS     |
| `assets.js_integrity`        | String | SHA-384 integrity hash of the bundled JS       |

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

See [SEO](/features/seo/) for more details on SRI and other security features.

## Render Hooks

Mythic supports render hooks that let you customize how images and links are rendered in your Markdown content. Define hook templates that override the default HTML output.

### Image Render Hook

Create `templates/render/image.tera.html` to customize image rendering:

```html
{# templates/render/image.tera.html #}
<figure>
  <img src="{{ src }}" alt="{{ alt }}" loading="lazy">
  {% if alt %}
  <figcaption>{{ alt }}</figcaption>
  {% endif %}
</figure>
```

Available variables in the image render hook:

| Variable | Type   | Description                        |
|----------|--------|------------------------------------|
| `src`    | String | The image source URL               |
| `alt`    | String | The alt text from Markdown         |
| `title`  | String | The optional title attribute       |

### Link Render Hook

Create `templates/render/link.tera.html` to customize link rendering:

```html
{# templates/render/link.tera.html #}
{% if src is starting_with("http") %}
<a href="{{ src }}" target="_blank" rel="noopener noreferrer">{{ text }}</a>
{% else %}
<a href="{{ src }}">{{ text }}</a>
{% endif %}
```

Available variables in the link render hook:

| Variable | Type   | Description                        |
|----------|--------|------------------------------------|
| `src`    | String | The link URL                       |
| `text`   | String | The link text content              |
| `title`  | String | The optional title attribute       |

## Related Content

Mythic can suggest related content for each page based on shared tags and categories. Related pages are available in templates:

```html
{% if page.related %}
<aside>
  <h2>Related Posts</h2>
  <ul>
    {% for related in page.related %}
    <li><a href="{{ related.path }}">{{ related.title }}</a></li>
    {% endfor %}
  </ul>
</aside>
{% endif %}
```

Each item in `page.related` has the same fields as a regular page object (`title`, `path`, `date`, `tags`, etc.). Mythic scores relatedness by counting shared taxonomy terms and returns the top matches.

## Remote Data

The `data.remote` object provides access to data fetched from external URLs, configured via `[[remote]]` in `mythic.toml`. Remote data is fetched at build time and cached based on its TTL.

```toml
# mythic.toml
[[remote]]
name = "github_repos"
url = "https://api.github.com/users/myuser/repos"
ttl = 3600

[[remote]]
name = "quotes"
url = "https://api.example.com/quotes.json"
ttl = 86400
```

Access remote data in templates:

```html
{% for repo in data.remote.github_repos %}
<a href="{{ repo.html_url }}">{{ repo.name }}</a>
<p>{{ repo.description }}</p>
{% endfor %}

{% for quote in data.remote.quotes %}
<blockquote>{{ quote.text }} &mdash; {{ quote.author }}</blockquote>
{% endfor %}
```

| Variable               | Type   | Description                                  |
|------------------------|--------|----------------------------------------------|
| `data.remote.<name>`   | Object | Parsed JSON response from the remote URL     |

The `ttl` (time to live) is in seconds. During the TTL window, Mythic serves the cached response instead of making a new HTTP request. Set `ttl = 0` to fetch on every build.

## Build JSON Output

Use `mythic build --json` for structured output in CI:

```json
{"total_pages":42,"pages_written":42,"pages_unchanged":0,"pages_skipped":2,"elapsed_ms":156}
```
