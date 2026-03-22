---
title: "Search"
---

# Search

Mythic can generate a search index at build time, enabling fast client-side search with no server or external service required.

## Enabling the Search Index

Enable search index generation in `mythic.toml`:

```toml
[plugins.search]
enable = true
```

When enabled, Mythic writes a `search-index.json` file to the root of your output directory on every build.

## Index Format

The generated `search-index.json` is an array of objects, one per page:

```json
[
  {
    "title": "Getting Started with Mythic",
    "slug": "getting-started-with-mythic",
    "url": "/blog/getting-started-with-mythic/",
    "summary": "A quick introduction to building sites with Mythic.",
    "tags": ["mythic", "tutorial"]
  },
  {
    "title": "Advanced Templates",
    "slug": "advanced-templates",
    "url": "/blog/advanced-templates/",
    "summary": "Deep dive into Tera and Handlebars template techniques.",
    "tags": ["templates", "tera"]
  }
]
```

| Field     | Source                                           |
|-----------|--------------------------------------------------|
| `title`   | Page `title` from frontmatter                    |
| `slug`    | Page slug (from frontmatter or filename)         |
| `url`     | Full URL path to the page                        |
| `summary` | Page `description` from frontmatter, or auto-generated from the first paragraph |
| `tags`    | Array of tag strings from frontmatter (empty array if none) |

Draft pages and pages with `sitemap.disable: true` are excluded from the index.

## Client-Side Search with Fuse.js

The index format is directly compatible with [Fuse.js](https://fusejs.io/), a lightweight fuzzy-search library. Here is a minimal example:

```html
<input type="search" id="search-input" placeholder="Search...">
<ul id="search-results"></ul>

<script src="https://cdn.jsdelivr.net/npm/fuse.js@7/dist/fuse.min.js"></script>
<script>
  let fuse;

  fetch('/search-index.json')
    .then(res => res.json())
    .then(data => {
      fuse = new Fuse(data, {
        keys: ['title', 'summary', 'tags'],
        threshold: 0.3,
      });
    });

  document.getElementById('search-input').addEventListener('input', function () {
    const results = fuse.search(this.value);
    const list = document.getElementById('search-results');
    list.innerHTML = results
      .slice(0, 10)
      .map(r => `<li><a href="${r.item.url}">${r.item.title}</a></li>`)
      .join('');
  });
</script>
```

## Client-Side Search with Lunr.js

The index is also compatible with [Lunr.js](https://lunrjs.com/). Build a Lunr index from the JSON data:

```html
<input type="search" id="search-input" placeholder="Search...">
<ul id="search-results"></ul>

<script src="https://cdn.jsdelivr.net/npm/lunr@2/lunr.min.js"></script>
<script>
  let lunrIndex;
  let documents;

  fetch('/search-index.json')
    .then(res => res.json())
    .then(data => {
      documents = data;
      lunrIndex = lunr(function () {
        this.ref('url');
        this.field('title', { boost: 10 });
        this.field('summary');
        this.field('tags');

        data.forEach(doc => {
          this.add({
            url: doc.url,
            title: doc.title,
            summary: doc.summary,
            tags: (doc.tags || []).join(' '),
          });
        });
      });
    });

  document.getElementById('search-input').addEventListener('input', function () {
    if (!this.value) return;
    const results = lunrIndex.search(this.value);
    const list = document.getElementById('search-results');
    list.innerHTML = results
      .slice(0, 10)
      .map(r => {
        const doc = documents.find(d => d.url === r.ref);
        return `<li><a href="${doc.url}">${doc.title}</a></li>`;
      })
      .join('');
  });
</script>
```

## Including Content in the Index

By default, the index includes only metadata (title, summary, tags) to keep the file small. To include the full rendered text content of each page for more accurate search results, set `index_content`:

```toml
[plugins.search]
enable = true
index_content = true
```

When enabled, each entry in the JSON gains a `content` field containing the plain-text body of the page (HTML tags stripped). Be aware that this increases the size of `search-index.json` significantly on large sites.
