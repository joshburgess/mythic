---
title: "Quickstart"
---

# Quickstart

This guide walks you through creating, developing, and building your first Mythic site.

## Step 1: Create a New Site

Use `mythic init` to scaffold a new project:

```bash
mythic init my-site
```

This creates the following structure:

```
my-site/
  mythic.toml
  content/
    index.md
  templates/
    base.tera.html
    page.tera.html
  static/
  styles/
    main.scss
  scripts/
  _data/
```

Move into your new project directory:

```bash
cd my-site
```

You can also specify a template engine when initializing:

```bash
mythic init my-site --engine handlebars
```

## Step 2: Start the Development Server

Launch the live-reloading development server:

```bash
mythic serve
```

Output:

```
  Loading site...
  Loaded 1 page in 4ms
  Server running at http://localhost:3000
  Watching for changes...
```

Open `http://localhost:3000` in your browser. You should see the default welcome page.

The server watches your files. Any changes to content, templates, styles, or data files trigger an automatic rebuild and browser reload.

To use a different port:

```bash
mythic serve --port 8080
```

To bind to all interfaces (useful in containers):

```bash
mythic serve --host 0.0.0.0
```

## Step 3: Create Content

The fastest way to create a new content file is with the `mythic new post` command:

```bash
mythic new post "My First Post"
```

This generates a file at `content/my-first-post.md` with pre-filled frontmatter (title, date, and draft status). You can also specify a section:

```bash
mythic new post "My First Post" --section blog
```

This creates `content/blog/my-first-post.md` instead. The `--section` flag creates the directory if it does not already exist.

Alternatively, you can create content files by hand. Create a new Markdown file in the `content/` directory:

```bash
mkdir -p content/blog
```

Create `content/blog/first-post.md` with the following:

```markdown
---
title: "My First Post"
date: 2026-03-21
tags:
  - intro
  - mythic
---

# Hello from Mythic

This is my first blog post built with Mythic. It supports all the
Markdown features you would expect:

- **Bold** and *italic* text
- [Links](https://mythic.site)
- Code blocks with syntax highlighting

Here is some Rust code:

\```rust
fn main() {
    println!("Hello from Mythic!");
}
\```
```

Save the file. If `mythic serve` is running, your browser reloads automatically and the new page appears at `/blog/first-post/`.

## Step 4: Edit Templates

Open `templates/base.tera.html` to customize the site layout:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{{ page.title }} | My Site</title>
    <link rel="stylesheet" href="{{ assets.css }}">
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/blog/">Blog</a>
    </nav>
    <main>
        {% block content %}{% endblock %}
    </main>
    <footer>
        <p>Built with Mythic</p>
    </footer>
</body>
</html>
```

And `templates/page.tera.html`:

```html
{% extends "base.tera.html" %}

{% block content %}
<article>
    <h1>{{ page.title }}</h1>
    {% if page.date %}
    <time datetime="{{ page.date }}">{{ page.date | date(format="%B %e, %Y") }}</time>
    {% endif %}
    <div class="content">
        {{ content | safe }}
    </div>
</article>
{% endblock %}
```

## Step 5: Add Data

Create a data file at `_data/site.yaml`:

```yaml
author: "Your Name"
description: "A site built with Mythic"
social:
  github: "https://github.com/yourusername"
  twitter: "https://twitter.com/yourusername"
```

Access this data in any template:

```html
<p>By {{ data.site.author }}</p>
<a href="{{ data.site.social.github }}">GitHub</a>
```

## Step 6: Build for Production

When you are ready to deploy, run the production build:

```bash
mythic build
```

Output:

```
  Loading site...
  Loaded 2 pages in 6ms
  Rendered templates in 12ms
  Processed assets in 28ms
  Built site in 46ms
  Output: public/
```

The generated site is written to the `public/` directory by default. This directory contains static HTML, CSS, JavaScript, and assets ready to deploy to any hosting provider.

To specify a different output directory:

```bash
mythic build --output dist/
```

To include draft posts in the build:

```bash
mythic build --drafts
```

## Step 7: Preview the Production Build

You can serve the built output to verify everything looks correct:

```bash
mythic serve --production
```

This serves the `public/` directory without watching or live reload, matching what users will see in production.

## Next Steps

- Learn about the [Project Structure](/getting-started/project-structure/) in detail
- Explore [Markdown features](/content/markdown/) and [Frontmatter](/content/frontmatter/)
- Customize your [Templates](/templates/tera/)
- Set up [Deployment](/deployment/github-pages/)
