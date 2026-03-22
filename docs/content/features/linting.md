---
title: "Content Linting"
---

# Content Linting

Mythic includes a built-in content linting system that checks your Markdown files for common issues during every build. Lint rules help maintain consistent quality across your site by enforcing word count ranges, required frontmatter fields, and structural conventions.

## Configuration

Enable and configure linting in the `[lint]` section of `mythic.toml`:

```toml
[lint]
min_word_count = 100
max_word_count = 10000
required_fields = ["title", "date", "description"]
require_tags = true
require_date = true
```

## Lint Rules

### min_word_count

Sets the minimum number of words a content page should have. Pages below this threshold trigger a warning. Useful for catching placeholder or stub pages that were accidentally left in.

```toml
[lint]
min_word_count = 100
```

Warning output:

```
  Warning: content/blog/stub-post.md has only 23 words (minimum: 100)
```

### max_word_count

Sets the upper bound for page word count. Pages exceeding this limit trigger a warning, helping you identify content that might benefit from being split into multiple pages.

```toml
[lint]
max_word_count = 5000
```

Warning output:

```
  Warning: content/blog/epic-guide.md has 7842 words (maximum: 5000)
```

### required_fields

A list of frontmatter fields that every content page must include. If any listed field is missing or empty, a warning is emitted.

```toml
[lint]
required_fields = ["title", "date", "description"]
```

Warning output:

```
  Warning: content/blog/my-post.md is missing required field: description
```

### require_tags

When set to `true`, every content page must have at least one tag defined in its frontmatter. This encourages consistent taxonomy usage across your site.

```toml
[lint]
require_tags = true
```

Warning output:

```
  Warning: content/blog/my-post.md has no tags
```

### require_date

When set to `true`, every content page must have a `date` field in its frontmatter. This is especially useful for blogs and news sites where chronological ordering matters.

```toml
[lint]
require_date = true
```

Warning output:

```
  Warning: content/blog/my-post.md has no date
```

## Orphan Page Detection

Mythic automatically detects orphan pages -- content pages that are not linked to from any other page on the site. Orphan pages are often the result of renaming or reorganizing content without updating internal links.

Orphan detection runs as part of the lint pass and produces warnings like:

```
  Warning: content/blog/old-draft.md is an orphan page (no incoming links)
```

Orphan detection only considers internal links within your content. External backlinks are not checked. Pages that serve as landing pages (such as `index.md` files) are excluded from orphan detection.

## How Warnings Appear

Lint warnings appear in the build output alongside other build messages. They do not cause the build to fail -- they are informational only.

```
  Loading site...
  Loaded 24 pages in 8ms
  Warning: content/blog/stub-post.md has only 23 words (minimum: 100)
  Warning: content/blog/my-post.md is missing required field: description
  Warning: content/about.md is an orphan page (no incoming links)
  Rendered templates in 18ms
  Built site in 52ms
```

In CI environments, use `mythic build --quiet` to suppress warnings and show only errors, or `mythic build --json` to get structured output that includes lint results as part of the JSON report.

## Full Example

A complete lint configuration for a blog:

```toml
[lint]
min_word_count = 200
max_word_count = 8000
required_fields = ["title", "date", "description"]
require_tags = true
require_date = true
```

This configuration ensures that every post has a title, date, and description, includes at least one tag, and falls within a reasonable word count range.
