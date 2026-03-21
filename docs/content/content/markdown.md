---
title: "Markdown"
---

# Markdown

Mythic uses an extended Markdown parser that supports GitHub Flavored Markdown (GFM) and several additional features. All content files in the `content/` directory are processed as Markdown.

## Basic Syntax

Standard Markdown works as expected:

```markdown
# Heading 1
## Heading 2
### Heading 3

Regular paragraph text with **bold**, *italic*, and `inline code`.

[Link text](https://example.com)

![Alt text](/images/photo.jpg)
```

## GitHub Flavored Markdown

Mythic supports the full GFM specification.

### Tables

```markdown
| Name    | Role       | Location   |
|---------|------------|------------|
| Alice   | Engineer   | New York   |
| Bob     | Designer   | London     |
| Carol   | Manager    | Tokyo      |
```

Column alignment is controlled with colons:

```markdown
| Left     | Center   | Right    |
|:---------|:--------:|---------:|
| aligned  | aligned  | aligned  |
```

### Task Lists

```markdown
- [x] Write the introduction
- [x] Add code examples
- [ ] Review and edit
- [ ] Publish
```

### Strikethrough

```markdown
This feature is ~~deprecated~~ no longer recommended.
```

### Autolinks

URLs and email addresses are automatically linked:

```markdown
Visit https://mythic.site for more info.
Contact support@mythic.site for help.
```

## Footnotes

Add footnotes with `[^label]` syntax:

```markdown
Mythic uses a fast Markdown parser[^1] written in Rust.

The parser supports all CommonMark features[^commonmark] plus extensions.

[^1]: Based on pulldown-cmark with custom extensions.
[^commonmark]: See https://commonmark.org for the specification.
```

Footnotes are rendered at the bottom of the page with back-references.

## Code Blocks with Syntax Highlighting

Fenced code blocks with language identifiers get automatic syntax highlighting:

````markdown
```rust
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```
````

Mythic supports syntax highlighting for over 150 languages, including:

- `rust`, `go`, `python`, `javascript`, `typescript`
- `html`, `css`, `scss`, `json`, `yaml`, `toml`
- `bash`, `sh`, `zsh`, `fish`
- `sql`, `graphql`
- `diff`, `markdown`

### Line Highlighting

Highlight specific lines by adding line numbers after the language:

````markdown
```rust {3,5-7}
use std::io;

fn main() {                    // highlighted
    let mut input = String::new();
    io::stdin()                // highlighted
        .read_line(&mut input) // highlighted
        .expect("read error"); // highlighted
    println!("You said: {input}");
}
```
````

### Line Numbers

Enable line numbers with the `linenos` attribute:

````markdown
```python linenos
def quicksort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    middle = [x for x in arr if x == pivot]
    right = [x for x in arr if x > pivot]
    return quicksort(left) + middle + quicksort(right)
```
````

### Titles

Add a title bar to code blocks:

````markdown
```toml title="mythic.toml"
[site]
title = "My Site"
base_url = "https://example.com"
```
````

## Heading Anchors

All headings automatically receive an `id` attribute based on the heading text, allowing direct linking:

```markdown
## My Section Title
```

Renders as:

```html
<h2 id="my-section-title">My Section Title</h2>
```

Link to it with `[jump to section](#my-section-title)`.

## Table of Contents

Mythic automatically generates a table of contents from your headings, available in templates as `{{ toc }}`. You can also insert it directly in Markdown:

```markdown
[[toc]]
```

This expands to a nested list of all headings in the document.

## Smart Typography

Mythic automatically converts:

| Input        | Output       |
|-------------|--------------|
| `"quotes"`  | "quotes"    |
| `'quotes'`  | 'quotes'    |
| `--`        | --           |
| `---`       | ---          |
| `...`       | ...          |

Disable smart typography in `mythic.toml` if you prefer raw characters:

```toml
[markdown]
smart_punctuation = false
```

## Raw HTML

You can include raw HTML in your Markdown files:

```markdown
This is a paragraph.

<div class="custom-banner">
  <p>This is raw HTML inside Markdown.</p>
</div>

Back to regular Markdown.
```

To disable raw HTML for security (e.g., user-contributed content):

```toml
[markdown]
allow_html = false
```

## Configuration

Control Markdown behavior in `mythic.toml`:

```toml
[markdown]
smart_punctuation = true
allow_html = true
heading_anchors = true
external_links_new_tab = true
syntax_theme = "onedark"
```

### Available Syntax Themes

- `onedark` (default)
- `github-light`
- `github-dark`
- `monokai`
- `dracula`
- `solarized-light`
- `solarized-dark`
- `nord`
- `base16-ocean`

You can also provide a custom TextMate `.tmTheme` file:

```toml
[markdown]
syntax_theme_path = "themes/custom.tmTheme"
```
