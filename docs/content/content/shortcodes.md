---
title: "Shortcodes"
---

# Shortcodes

Shortcodes let you embed rich components in your Markdown content without writing raw HTML. They are reusable template snippets invoked with a concise syntax inside your content files.

## Syntax

Mythic supports two shortcode forms: self-closing and paired.

### Self-Closing Shortcodes

Use `{%/* shortcode_name() */%}` for shortcodes that do not wrap content:

```markdown
{{/* youtube(id="dQw4w9WgXcQ") */}}
```

With multiple arguments:

```markdown
{{/* image(src="/images/photo.jpg", alt="A photo", width=800) */}}
```

### Paired Shortcodes

Use opening and closing tags for shortcodes that wrap content:

```markdown
{%/* callout(type="warning") */%}
Be careful when editing configuration files. A syntax error in
`mythic.toml` will prevent your site from building.
{%/* end */%}
```

The content between the tags is processed as Markdown and passed to the shortcode template as `{{ body }}`.

## Built-in Shortcodes

Mythic ships with several shortcodes ready to use.

### youtube

Embeds a YouTube video with a responsive wrapper:

```markdown
{{/* youtube(id="dQw4w9WgXcQ") */}}
```

Parameters:

| Parameter   | Required | Description                         |
|-------------|----------|-------------------------------------|
| `id`        | Yes      | The YouTube video ID                |
| `autoplay`  | No       | Autoplay the video (default: false) |
| `start`     | No       | Start time in seconds               |

### vimeo

Embeds a Vimeo video:

```markdown
{{/* vimeo(id="123456789") */}}
```

### figure

A figure with caption and optional attributes:

```markdown
{{/* figure(src="/images/chart.png", alt="Sales chart", caption="Q1 2026 sales data") */}}
```

Parameters:

| Parameter  | Required | Description                   |
|------------|----------|-------------------------------|
| `src`      | Yes      | Image source path             |
| `alt`      | No       | Alt text for accessibility    |
| `caption`  | No       | Caption displayed below image |
| `width`    | No       | Width in pixels or percentage |
| `class`    | No       | CSS class name                |

### callout

Styled callout boxes for notes, warnings, and tips:

```markdown
{%/* callout(type="note") */%}
This is a note with additional context for the reader.
{%/* end */%}

{%/* callout(type="warning") */%}
This action cannot be undone. Make sure you have a backup.
{%/* end */%}

{%/* callout(type="tip") */%}
Use `mythic serve` during development for live reload.
{%/* end */%}

{%/* callout(type="danger") */%}
Never commit secrets to your repository.
{%/* end */%}
```

Types: `note`, `tip`, `warning`, `danger`, `info`.

### details

A collapsible disclosure element:

```markdown
{%/* details(summary="Click to expand") */%}
This content is hidden by default and revealed when the user clicks the summary.

- Supports full Markdown inside
- Including lists and code blocks
{%/* end */%}
```

## Creating Custom Shortcodes

Custom shortcodes are template files placed in `templates/shortcodes/`. The filename (without extension) becomes the shortcode name.

### Example: GitHub Gist

Create `templates/shortcodes/gist.tera.html`:

```html
<script src="https://gist.github.com/{{ id }}.js"></script>
```

Use it in content:

```markdown
{{/* gist(id="user/abc123def456") */}}
```

### Example: Styled Code Block with Title

Create `templates/shortcodes/code.tera.html`:

```html
<div class="code-block">
    {% if title %}
    <div class="code-title">{{ title }}</div>
    {% endif %}
    <div class="code-content">
        {{ body | safe }}
    </div>
</div>
```

Use it in content:

```markdown
{%/* code(title="Configuration") */%}
```toml
[site]
title = "My Site"
```
{%/* end */%}
```

### Example: Two-Column Layout

Create `templates/shortcodes/columns.tera.html`:

```html
<div class="columns" style="display: grid; grid-template-columns: {{ left_width | default(value="1fr") }} {{ right_width | default(value="1fr") }}; gap: 2rem;">
    {{ body | safe }}
</div>
```

### Accessing Parameters

All parameters passed to the shortcode are available as template variables:

```markdown
{{/* myshortcode(name="Alice", role="Engineer", highlight=true) */}}
```

In the template:

```html
<!-- templates/shortcodes/myshortcode.tera.html -->
<div class="card {% if highlight %}highlighted{% endif %}">
    <h3>{{ name }}</h3>
    <p>{{ role }}</p>
</div>
```

### The body Variable

For paired shortcodes, `{{ body }}` contains the rendered Markdown content between the opening and closing tags. Always use `{{ body | safe }}` since the body has already been converted to HTML.

## Shortcode Resolution Order

When Mythic encounters a shortcode, it looks for a matching template in this order:

1. `templates/shortcodes/{name}.tera.html` (or `.hbs.html`)
2. Built-in shortcodes bundled with Mythic

If no matching template is found, the build fails with an error indicating the unknown shortcode name and the file where it was used.

## Escaping Shortcodes

To display shortcode syntax literally without processing, wrap it in a raw block:

```markdown
{% raw %}
{{/* youtube(id="example") */}}
{% endraw %}
```

Or use an HTML comment to break the syntax:

```markdown
{<!-- -->{/* youtube(id="example") */}}
```
