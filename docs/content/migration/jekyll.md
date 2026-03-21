---
title: "Migrating from Jekyll"
---

# Migrating from Jekyll

Mythic includes a built-in migration tool that converts Jekyll sites to Mythic's structure. It handles most of the conversion automatically, but some manual adjustments may be needed.

## Running the Migration

From your Jekyll project directory:

```bash
mythic migrate --from jekyll --source . --output ../my-mythic-site
```

Or point to a Jekyll site from anywhere:

```bash
mythic migrate --from jekyll --source /path/to/jekyll-site --output /path/to/mythic-site
```

### Options

| Flag         | Default | Description                                    |
|--------------|---------|------------------------------------------------|
| `--source`   | `.`     | Path to the Jekyll project                     |
| `--output`   | `./out` | Path for the new Mythic project                |
| `--dry-run`  | `false` | Preview changes without writing files          |
| `--verbose`  | `false` | Show detailed conversion output                |

Preview what will be converted:

```bash
mythic migrate --from jekyll --source . --output ../mythic-site --dry-run
```

## What Gets Converted

### Content Files

| Jekyll                          | Mythic                              |
|---------------------------------|-------------------------------------|
| `_posts/2026-03-21-my-post.md` | `content/blog/my-post.md`           |
| `_pages/about.md`              | `content/about.md`                  |
| `index.md`                     | `content/index.md`                  |
| `_drafts/wip.md`               | `content/blog/wip.md` (draft: true) |

The date is extracted from the Jekyll filename and placed into the frontmatter. The date prefix is removed from the filename.

### Frontmatter

Jekyll frontmatter is converted to Mythic's format:

```yaml
# Jekyll
---
layout: post
title: "My Post"
date: 2026-03-21 14:30:00 -0500
categories: tutorials rust
tags:
  - beginner
  - rust
permalink: /blog/:title/
---

# Mythic (converted)
---
title: "My Post"
date: 2026-03-21T14:30:00-05:00
layout: blog
categories:
  - tutorials
  - rust
tags:
  - beginner
  - rust
slug: "my-post"
---
```

Key conversions:

- `layout: post` becomes `layout: blog`
- `categories` as a space-separated string becomes an array
- `permalink` is converted to a `slug` where possible
- `published: false` becomes `draft: true`

### Configuration

`_config.yml` is converted to `mythic.toml`:

```yaml
# Jekyll _config.yml
title: My Site
description: A Jekyll site
url: https://example.com
baseurl: /blog
markdown: kramdown
permalink: /:categories/:title/

plugins:
  - jekyll-feed
  - jekyll-sitemap
```

```toml
# Mythic mythic.toml (converted)
[site]
title = "My Site"
description = "A Jekyll site"
base_url = "https://example.com/blog"
generate_feed = true

[sitemap]
enable = true

[build]
template_engine = "tera"
```

### Template Syntax Changes

| Jekyll (Liquid)                        | Mythic (Tera)                                    |
|----------------------------------------|--------------------------------------------------|
| `{{ content }}`                        | `{{ content \| safe }}`                          |
| `{{ page.url }}`                       | `{{ page.path }}`                                |
| `{{ site.posts }}`                     | `{{ site.sections.blog }}`                       |
| `{% for post in site.posts %}`         | `{% for post in site.sections.blog %}`           |
| `{% elsif %}`                          | `{% elif %}`                                     |
| `{% assign x = 1 %}`                  | `{% set x = 1 %}`                               |
| `{% include file.html %}`             | `{% include "partials/file.tera.html" %}`        |
| `{{ page.date \| date: "%B %d, %Y" }}`| `{{ page.date \| date(format="%B %d, %Y") }}`   |
| `{% capture var %}...{% endcapture %}` | `{% set var = ... %}`                            |

### Data Files

| Jekyll             | Mythic              |
|--------------------|---------------------|
| `_data/nav.yml`    | `_data/nav.yaml`    |
| `_data/authors.json` | `_data/authors.json` |
| `_data/team.csv`   | Not converted (manual) |

CSV data files are not supported in Mythic. Convert them to YAML or JSON manually.

### Static Files

| Jekyll                 | Mythic                 |
|------------------------|------------------------|
| `assets/images/`       | `static/images/`       |
| `assets/css/`          | `styles/` or `static/` |
| `assets/js/`           | `scripts/` or `static/`|
| `favicon.ico`          | `static/favicon.ico`   |

SCSS files in Jekyll's `assets/css/` are moved to `styles/`. Plain CSS and pre-built files go to `static/`.

### Layouts and Includes

| Jekyll               | Mythic                            |
|-----------------------|-----------------------------------|
| `_layouts/`           | `templates/`                      |
| `_includes/`          | `templates/partials/`             |

The migration tool copies layout and include files but performs only basic Liquid-to-Tera syntax conversion. Complex Liquid logic requires manual review.

### Collections

Jekyll collections are converted to content sections:

```yaml
# Jekyll _config.yml
collections:
  projects:
    output: true
    permalink: /projects/:title/
```

```
# Jekyll                    -> Mythic
_projects/my-project.md     -> content/projects/my-project.md
```

## What Requires Manual Conversion

### Liquid Templates

The migration tool handles basic syntax conversions, but complex Liquid constructs need manual attention:

- `{% capture %}` blocks with complex logic
- Liquid filters like `| where:`, `| sort:`, `| group_by:`
- `{% case %}` / `{% when %}` (use `{% if %}` / `{% elif %}` in Tera)
- `forloop.first` / `forloop.last` (use `loop.first` / `loop.last` in Tera)

### Jekyll Plugins

Jekyll plugins do not work in Mythic. Common replacements:

| Jekyll Plugin          | Mythic Equivalent                          |
|------------------------|--------------------------------------------|
| `jekyll-feed`          | Built-in: `generate_feed = true`           |
| `jekyll-sitemap`       | Built-in: `[sitemap] enable = true`        |
| `jekyll-seo-tag`       | Manual: add meta tags in base template     |
| `jekyll-paginate`      | Built-in: taxonomy pagination              |
| `jekyll-archives`      | Built-in: taxonomies                       |
| `jekyll-redirect-from` | Frontmatter: `aliases`                     |
| `jekyll-sass-converter`| Built-in: Sass/SCSS support                |

### Liquid Highlight Tags

```liquid
{% highlight ruby %}
def hello
  puts "Hello"
end
{% endhighlight %}
```

Convert to fenced code blocks:

````markdown
```ruby
def hello
  puts "Hello"
end
```
````

### Gem-Based Themes

If your Jekyll site uses a gem-based theme, the theme's layouts and includes are not in your project directory. You need to extract them first:

```bash
# Find the theme's location
bundle show minima

# Copy layouts and includes into your project
cp -r $(bundle show minima)/_layouts/ _layouts/
cp -r $(bundle show minima)/_includes/ _includes/
```

Then run the migration.

### Post Excerpts

Jekyll's `{{ post.excerpt }}` is replaced by `{{ post.description }}`. Add a `description` field to your frontmatter, or use the `truncate` filter on content.

## Step-by-Step Migration

1. **Run the migration tool:**
   ```bash
   mythic migrate --from jekyll --source . --output ../mythic-site
   ```

2. **Review the migration report** printed to the console for warnings.

3. **Create or revise templates** in `templates/` based on your Jekyll layouts. Start with `base.tera.html` and `page.tera.html`.

4. **Test the build:**
   ```bash
   cd ../mythic-site
   mythic build
   ```

5. **Fix any errors** shown in the build output. Most will be template-related.

6. **Start the dev server** and compare with your Jekyll site:
   ```bash
   mythic serve
   ```

7. **Check every page** for visual or content differences.
