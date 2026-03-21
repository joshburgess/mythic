---
title: "Handlebars Templates"
---

# Handlebars Templates

Mythic supports Handlebars as an alternative template engine. Handlebars emphasizes logic-less templates with a clean syntax and powerful helper system.

Handlebars template files use the `.hbs.html` extension.

## Basic Expressions

Output values with double curly braces:

```html
<h1>{{page.title}}</h1>
<p>{{page.description}}</p>
<p>Published by {{page.extra.author}}</p>
```

All expressions are HTML-escaped by default for security.

## Raw HTML with Triple-Stash

Use triple curly braces to output raw, unescaped HTML:

```html
<!-- Double curly: HTML-escaped output -->
{{page.description}}

<!-- Triple curly: raw HTML output -->
{{{content}}}
```

Always use `{{{content}}}` for rendering page content, since it has already been converted from Markdown to HTML.

## Helpers

### if / else

```html
{{#if page.draft}}
  <span class="badge">Draft</span>
{{else}}
  <span class="badge">Published</span>
{{/if}}
```

Falsy values: `false`, `undefined`, `null`, `""`, `0`, and empty arrays `[]`.

Nested conditions with `else if`:

```html
{{#if page.extra.featured}}
  <div class="featured-post">
{{else if page.extra.pinned}}
  <div class="pinned-post">
{{else}}
  <div class="post">
{{/if}}
    <h2>{{page.title}}</h2>
  </div>
```

### unless

The inverse of `if`:

```html
{{#unless page.draft}}
  <a href="{{page.path}}">{{page.title}}</a>
{{/unless}}
```

### each

Iterate over arrays:

```html
<ul>
{{#each page.tags}}
  <li>{{this}}</li>
{{/each}}
</ul>
```

Inside an `each` block, special variables are available:

```html
{{#each site.pages}}
  {{@index}}    <!-- 0-based index -->
  {{@first}}    <!-- true on first iteration -->
  {{@last}}     <!-- true on last iteration -->
  {{@key}}      <!-- key name when iterating objects -->
  {{this.title}} <!-- current item -->
{{/each}}
```

Iterate over objects:

```html
{{#each data.site.social}}
  <a href="{{this}}">{{@key}}</a>
{{/each}}
```

Empty fallback:

```html
{{#each site.pages}}
  <li>{{this.title}}</li>
{{else}}
  <li>No pages found.</li>
{{/each}}
```

### with

Change the context for a block:

```html
{{#with page.extra}}
  <p>Author: {{author}}</p>
  <img src="{{cover_image}}" alt="Cover">
{{/with}}
```

`with` also supports an `else` for when the value is falsy:

```html
{{#with page.extra.author}}
  <p>By {{this}}</p>
{{else}}
  <p>By Anonymous</p>
{{/with}}
```

### lookup

Access dynamic keys or array indices:

```html
{{lookup page.tags 0}}
{{lookup data.authors page.extra.author_id}}
```

### log

Output debug information to the build console:

```html
{{log page.title}}
{{log "Current path:" page.path}}
```

## Built-in Helpers

Mythic registers several custom helpers for common operations.

### eq / ne / gt / lt / gte / lte

Comparison helpers for use in `if` blocks:

```html
{{#if (eq page.layout "blog")}}
  <div class="blog-layout">
{{/if}}

{{#if (gt page.extra.word_count 1000)}}
  <span>Long read</span>
{{/if}}
```

### and / or / not

Logical operators:

```html
{{#if (and page.date (not page.draft))}}
  <time>{{page.date}}</time>
{{/if}}

{{#if (or page.extra.featured page.extra.pinned)}}
  <span class="highlight">Highlighted</span>
{{/if}}
```

### date_format

Format dates:

```html
{{date_format page.date "%B %e, %Y"}}
<!-- Output: March 21, 2026 -->

{{date_format page.date "%Y-%m-%d"}}
<!-- Output: 2026-03-21 -->
```

### truncate

Truncate strings:

```html
{{truncate page.description 100}}
```

### slugify

Convert a string to a URL slug:

```html
<a href="/tags/{{slugify tag}}/">{{tag}}</a>
```

### json

Serialize a value to JSON:

```html
<script>
  const siteData = {{{json data.site}}};
</script>
```

Note the triple curly braces to output raw JSON without HTML escaping.

## Partials

Partials are reusable template fragments. Place them in `templates/partials/`.

### Registering Partials

Any `.hbs.html` file in `templates/partials/` is automatically registered:

```
templates/
  partials/
    header.hbs.html
    footer.hbs.html
    post-card.hbs.html
```

### Using Partials

Include a partial with `{{> partial_name}}`:

```html
{{> header}}

<main>
  {{{content}}}
</main>

{{> footer}}
```

### Partial Parameters

Pass additional data to partials:

```html
{{> post-card post=this featured=true}}
```

Inside the partial:

```html
<!-- templates/partials/post-card.hbs.html -->
<article class="{{#if featured}}featured{{/if}}">
    <h2><a href="{{post.path}}">{{post.title}}</a></h2>
    <p>{{post.description}}</p>
</article>
```

### Inline Partials

Define partials inline within a template:

```html
{{#*inline "sidebar"}}
  <aside>
    <h3>Recent Posts</h3>
    <ul>
      {{#each site.pages}}
        <li><a href="{{this.path}}">{{this.title}}</a></li>
      {{/each}}
    </ul>
  </aside>
{{/inline}}

{{> sidebar}}
```

## Layout Inheritance

Handlebars uses partials for layout composition. Define a layout partial and use a body block:

```html
<!-- templates/partials/layout.hbs.html -->
<!DOCTYPE html>
<html>
<head>
    <title>{{page.title}} | {{site.title}}</title>
</head>
<body>
    {{> header}}
    <main>
        {{> @partial-block}}
    </main>
    {{> footer}}
</body>
</html>
```

Use the layout in a page template:

```html
<!-- templates/page.hbs.html -->
{{#> layout}}
  <article>
    <h1>{{page.title}}</h1>
    {{{content}}}
  </article>
{{/layout}}
```

The content between `{{#> layout}}` and `{{/layout}}` replaces `{{> @partial-block}}` in the layout partial.

## Comments

Handlebars comments are excluded from output:

```html
{{!-- This is a comment --}}
{{! Short comment syntax }}
```

## Escaping

To output literal curly braces, use a raw block:

```html
{{{{raw}}}}
  {{this is not processed}}
{{{{/raw}}}}
```

## Choosing Between Tera and Handlebars

| Feature              | Tera               | Handlebars          |
|----------------------|---------------------|---------------------|
| Logic in templates   | Full expressions    | Helper-based        |
| Template inheritance | `extends` / `block` | Partial blocks     |
| Macros               | Yes                 | No (use partials)   |
| Filters              | Pipe syntax         | Helper syntax       |
| Learning curve       | Moderate            | Low                 |

You can use both engines in the same project. Each template file's extension determines which engine renders it.
