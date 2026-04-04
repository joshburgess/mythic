---
title: "Tera Templates"
---

# Tera Templates

Tera is the default template engine in Mythic. It is inspired by Jinja2 and Django templates, offering a familiar syntax with powerful features like template inheritance, filters, and macros.

Tera template files use the `.tera.html` extension.

## Variables

Output variables with double curly braces:

```html
<h1>{{ page.title }}</h1>
<p>Published on {{ page.date }}</p>
<p>{{ site.title }}</p>
```

Access nested values with dot notation:

```html
<span>{{ page.extra.author }}</span>
<a href="{{ data.site.social.github }}">GitHub</a>
```

Array access by index:

```html
<p>First tag: {{ page.tags[0] }}</p>
```

## Filters

Filters transform values. Apply them with the pipe `|` operator:

```html
{{ page.title | upper }}
{{ page.title | lower }}
{{ page.title | truncate(length=50) }}
{{ page.date | date(format="%B %e, %Y") }}
{{ page.content | wordcount }}
{{ page.description | default(value="No description available.") }}
```

### Common Filters

| Filter         | Description                             | Example                                       |
|----------------|-----------------------------------------|-----------------------------------------------|
| `upper`        | Convert to uppercase                    | `{{ name | upper }}`                          |
| `lower`        | Convert to lowercase                    | `{{ name | lower }}`                          |
| `trim`         | Remove leading/trailing whitespace      | `{{ text | trim }}`                           |
| `truncate`     | Truncate to length                      | `{{ text | truncate(length=100) }}`           |
| `replace`      | Replace substring                       | `{{ text | replace(from="old", to="new") }}`  |
| `date`         | Format a date                           | `{{ date | date(format="%Y-%m-%d") }}`        |
| `default`      | Fallback value                          | `{{ val | default(value="none") }}`           |
| `length`       | Length of string or array               | `{{ items | length }}`                        |
| `reverse`      | Reverse a string or array               | `{{ items | reverse }}`                       |
| `sort_by`      | Sort array of objects by key            | `{{ posts | sort_by(attribute="date") }}`     |
| `first`        | First element of an array               | `{{ items | first }}`                         |
| `last`         | Last element of an array                | `{{ items | last }}`                          |
| `join`         | Join array elements                     | `{{ tags | join(sep=", ") }}`                 |
| `safe`         | Mark as safe HTML (no escaping)         | `{{ content | safe }}`                        |
| `striptags`    | Remove HTML tags                        | `{{ content | striptags }}`                   |
| `slugify`      | Convert to URL slug                     | `{{ title | slugify }}`                       |
| `json_encode`  | Serialize to JSON                       | `{{ data | json_encode }}`                    |

### The safe Filter

By default, Tera escapes HTML in all variable output. Use the `safe` filter when you explicitly want to render HTML:

```html
<!-- Escaped (default) - HTML tags shown as text -->
{{ content }}

<!-- Unescaped - HTML rendered in the browser -->
{{ content | safe }}
```

Always use `{{ content | safe }}` for page content, since it has already been converted from Markdown to HTML.

## Control Structures

### if / elif / else

```html
{% if page.draft %}
  <span class="badge">Draft</span>
{% elif page.date > now() %}
  <span class="badge">Scheduled</span>
{% else %}
  <span class="badge">Published</span>
{% endif %}
```

Truthiness: empty strings, `0`, `false`, empty arrays, and `null` are falsy.

### for Loops

```html
<ul>
{% set all_pages = get_pages() %}
{% for post in all_pages %}
  <li>
    <a href="{{ post.url }}">{{ post.title }}</a>
  </li>
{% endfor %}
</ul>
```

Loop variables:

```html
{% for item in items %}
  {{ loop.index }}      <!-- 1-based index -->
  {{ loop.index0 }}     <!-- 0-based index -->
  {{ loop.first }}      <!-- true on first iteration -->
  {{ loop.last }}       <!-- true on last iteration -->
{% endfor %}
```

Filtering and sorting in loops:

```html
{% set all_pages = get_pages() %}
{% for post in all_pages | sort_by(attribute="date") | reverse %}
  {% if not post.draft %}
    <article>
      <h2>{{ post.title }}</h2>
      <time>{{ post.date | date(format="%Y-%m-%d") }}</time>
    </article>
  {% endif %}
{% endfor %}
```

Empty loop fallback:

```html
{% set all_pages = get_pages() %}
{% for post in all_pages %}
  <li>{{ post.title }}</li>
{% else %}
  <li>No posts found.</li>
{% endfor %}
```

### set

Assign values to variables:

```html
{% set full_title = page.title ~ " | " ~ site.title %}
<title>{{ full_title }}</title>

{% set show_sidebar = page.extra.sidebar | default(value=true) %}
```

## Template Inheritance

Template inheritance is the primary way to build layouts in Tera.

### Base Template

```html
<!-- templates/base.tera.html -->
<!DOCTYPE html>
<html lang="{{ site.language | default(value='en') }}">
<head>
    <meta charset="utf-8">
    <title>{% block title %}{{ page.title }} | {{ site.title }}{% endblock %}</title>
    {% block head %}{% endblock %}
</head>
<body>
    <header>
        {% block header %}
        <nav>
            {% for item in data.nav.main %}
            <a href="{{ item.url }}">{{ item.label }}</a>
            {% endfor %}
        </nav>
        {% endblock %}
    </header>
    <main>
        {% block content %}{% endblock %}
    </main>
    <footer>
        {% block footer %}
        <p>&copy; {{ now() | date(format="%Y") }} {{ site.title }}</p>
        {% endblock %}
    </footer>
</body>
</html>
```

### Child Template

```html
<!-- templates/page.tera.html -->
{% extends "base.tera.html" %}

{% block content %}
<article>
    <h1>{{ page.title }}</h1>
    {{ content | safe }}
</article>
{% endblock %}
```

### Calling the Parent Block

Use `{{ super() }}` to include the parent block's content:

```html
{% block head %}
{{ super() }}
<link rel="stylesheet" href="/css/custom.css">
{% endblock %}
```

## Includes

Include reusable template fragments:

```html
{% include "partials/header.tera.html" %}

<main>{{ content | safe }}</main>

{% include "partials/footer.tera.html" %}
```

Includes share the current template context, so they can access all the same variables.

## Macros

Define reusable snippets with parameters:

```html
<!-- templates/macros.tera.html -->
{% macro card(title, description, url) %}
<div class="card">
    <h3><a href="{{ url }}">{{ title }}</a></h3>
    <p>{{ description }}</p>
</div>
{% endmacro %}

{% macro tag_list(tags) %}
<ul class="tags">
    {% for tag in tags %}
    <li><a href="/tags/{{ tag | slugify }}/">{{ tag }}</a></li>
    {% endfor %}
</ul>
{% endmacro %}
```

Import and use macros:

```html
{% import "macros.tera.html" as macros %}

{{ macros::card(title="My Post", description="A great post.", url="/blog/my-post/") }}
{{ macros::tag_list(tags=page.tags) }}
```

## Whitespace Control

Add a minus sign to trim whitespace:

```html
{%- if condition -%}
    trimmed
{%- endif -%}
```

This removes all whitespace before `{%-` and after `-%}`.

## Comments

Template comments are not included in the output:

```html
{# This is a comment and will not appear in the HTML #}

{#
  Multi-line comments
  are also supported.
#}
```

## Raw Blocks

Output Tera syntax literally without processing:

```html
{% raw %}
  {{ this will not be processed }}
  {% neither will this %}
{% endraw %}
```

This is useful for documentation or when embedding client-side template syntax.

## Custom Filters

In addition to the built-in Tera filters, Mythic provides several custom filters for common content operations:

### reading_time

Estimates the reading time for a block of content, assuming an average reading speed of 200 words per minute:

```html
{{ content | reading_time }}
```

Output: `3 min read`

### word_count

Returns the total number of words in the content:

```html
{{ content | word_count }}
```

Output: `342`

### truncate_words

Truncates content to a specified number of words and appends an ellipsis:

```html
{{ content | truncate_words(count=20) }}
```

Output: the first 20 words followed by `...`

These filters work on any string value, so you can use them with page content, descriptions, or any other text variable.
