//! Markdown-to-HTML rendering with syntax highlighting and TOC extraction.

use crate::highlight::Highlighter;
use crate::toc;
use mythic_core::page::Page;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use rayon::prelude::*;

/// Configuration for the markdown renderer.
pub struct RenderConfig {
    pub highlight_theme: String,
    pub line_numbers: bool,
    pub toc_min_level: u32,
    pub toc_max_level: u32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        RenderConfig {
            highlight_theme: "base16-ocean.dark".to_string(),
            line_numbers: false,
            toc_min_level: 2,
            toc_max_level: 4,
        }
    }
}

/// Render markdown to HTML for all pages in parallel, with highlighting and TOC.
pub fn render_markdown(pages: &mut [Page]) {
    render_markdown_with_config(pages, &RenderConfig::default());
}

/// Render markdown with explicit configuration.
pub fn render_markdown_with_config(pages: &mut [Page], config: &RenderConfig) {
    let highlighter = Highlighter::new(&config.highlight_theme, config.line_numbers);
    let min = config.toc_min_level;
    let max = config.toc_max_level;

    pages.par_iter_mut().for_each(|page| {
        let html = render_one_highlighted(&page.raw_content, &highlighter);
        let (toc_entries, html_with_ids) = toc::extract_toc(&html, min, max);
        page.rendered_html = Some(html_with_ids);
        page.toc = toc_entries;
    });
}

/// Render a single markdown string to HTML (no highlighting, for backwards compat).
pub fn render_one(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, opts);
    let mut html_output = String::with_capacity(markdown.len() * 2);
    pulldown_cmark::html::push_html(&mut html_output, parser);
    transform_admonitions(&html_output)
}

/// Render markdown with syntax highlighting for fenced code blocks.
fn render_one_highlighted(markdown: &str, highlighter: &Highlighter) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, opts);

    let mut html_output = String::with_capacity(markdown.len() * 2);
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();

    let mut events: Vec<Event> = Vec::new();

    for event in parser {
        match &event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_content.clear();
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                continue;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if !code_lang.is_empty() {
                    let highlighted = highlighter.highlight(&code_content, &code_lang);
                    events.push(Event::Html(highlighted.into()));
                } else {
                    // Plain code block
                    events.push(Event::Html(
                        format!("<pre><code>{}</code></pre>", escape_html(&code_content)).into(),
                    ));
                }
                continue;
            }
            Event::Text(text) if in_code_block => {
                code_content.push_str(text);
                continue;
            }
            _ => {}
        }

        events.push(event);
    }

    pulldown_cmark::html::push_html(&mut html_output, events.into_iter());
    transform_admonitions(&html_output)
}

/// Transform blockquote-based admonitions (e.g. `> [!NOTE]`) into styled divs.
///
/// Recognises the callout types NOTE, WARNING, TIP, IMPORTANT, and CAUTION.
/// A matching blockquote is replaced with:
///
/// ```html
/// <div class="admonition admonition-note">
/// <p class="admonition-title">Note</p>
/// ...remaining content...
/// </div>
/// ```
fn transform_admonitions(html: &str) -> String {
    const TYPES: &[&str] = &["NOTE", "WARNING", "TIP", "IMPORTANT", "CAUTION"];

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(bq_start) = remaining.find("<blockquote>") {
        // Push everything before this blockquote.
        result.push_str(&remaining[..bq_start]);

        // Find the matching closing tag.
        let after_open = &remaining[bq_start..];
        let Some(close_pos) = after_open.find("</blockquote>") else {
            // Malformed HTML — just push the rest and stop.
            result.push_str(&remaining[bq_start..]);
            remaining = "";
            break;
        };
        let bq_inner_start = "<blockquote>".len();
        let inner = &after_open[bq_inner_start..close_pos];
        let after_close = &after_open[close_pos + "</blockquote>".len()..];

        // Check if the inner content starts with a [!TYPE] marker.
        // The marker is typically inside a <p> tag: <p>[!NOTE]\ncontent</p> or
        // <p>[!NOTE]</p> followed by more content.
        let trimmed = inner.trim_start();
        let mut matched_type: Option<&str> = None;

        for ty in TYPES {
            let marker = format!("[!{}]", ty);
            // Patterns to match:
            // 1. <p>[!TYPE]\n  or  <p>[!TYPE]<br...  or <p>[!TYPE]</p>
            if let Some(p_start) = trimmed.find("<p>") {
                let after_p = &trimmed[p_start + 3..];
                let after_p_trimmed = after_p.trim_start();
                if after_p_trimmed.starts_with(&marker) {
                    matched_type = Some(ty);
                    break;
                }
            }
        }

        if let Some(ty) = matched_type {
            let ty_lower = ty.to_lowercase();
            // Capitalize first letter for the title.
            let title = {
                let mut chars = ty_lower.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            };

            let marker = format!("[!{}]", ty);

            // Remove the marker from the inner content.
            // Find where the marker is and strip it.
            let Some(marker_pos) = inner.find(&marker) else {
                // Marker confirmed above but not found in raw inner — keep original blockquote.
                result.push_str(&after_open[..close_pos + "</blockquote>".len()]);
                remaining = after_close;
                continue;
            };
            let before_marker = &inner[..marker_pos];
            let after_marker = &inner[marker_pos + marker.len()..];

            // Clean up: if the marker was followed by a newline or <br>, remove it.
            let after_marker = after_marker.strip_prefix('\n').unwrap_or(after_marker);

            // Reconstruct the cleaned inner content.
            let cleaned_inner = format!("{}{}", before_marker, after_marker);

            // Check if the cleaned inner has an empty leading <p></p> and remove it.
            let cleaned_inner = cleaned_inner.replace("<p>\n", "<p>").replace("<p></p>", "");

            // Trim leading/trailing whitespace from the cleaned inner.
            let cleaned_inner = cleaned_inner.trim();

            result.push_str(&format!(
                "<div class=\"admonition admonition-{}\">\n<p class=\"admonition-title\">{}</p>\n{}\n</div>",
                ty_lower, title, cleaned_inner
            ));
        } else {
            // Not an admonition, keep original blockquote.
            result.push_str(&after_open[..close_pos + "</blockquote>".len()]);
        }

        remaining = after_close;
    }

    result.push_str(remaining);
    result
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_markdown() {
        let html =
            render_one("# Hello\n\nA **bold** paragraph with *italics*.\n\n- item 1\n- item 2");
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italics</em>"));
        assert!(html.contains("<li>item 1</li>"));
    }

    #[test]
    fn gfm_extensions() {
        let md = "| Col A | Col B |\n|-------|-------|\n| 1     | 2     |\n\n~~struck~~\n\n- [x] done\n- [ ] todo";
        let html = render_one(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<del>struck</del>"));
        assert!(html.contains("type=\"checkbox\""));
    }

    #[test]
    fn empty_body() {
        let html = render_one("");
        assert!(html.is_empty());
    }

    #[test]
    fn highlighted_rust_code() {
        let highlighter = Highlighter::new("base16-ocean.dark", false);
        let md = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let html = render_one_highlighted(md, &highlighter);
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("<span"));
        assert!(html.contains("main"));
    }

    #[test]
    fn plain_code_block_no_lang() {
        let highlighter = Highlighter::new("base16-ocean.dark", false);
        let md = "```\njust plain text\n```";
        let html = render_one_highlighted(md, &highlighter);
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("just plain text"));
        // Should not have syntax spans
    }

    #[test]
    fn line_numbers_in_highlighted() {
        let highlighter = Highlighter::new("base16-ocean.dark", true);
        let md = "```rust\nline1\nline2\n```";
        let html = render_one_highlighted(md, &highlighter);
        assert!(html.contains("line-number"));
    }

    #[test]
    fn toc_populated_on_render() {
        let mut pages = vec![mythic_core::page::Page {
            source_path: std::path::PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: mythic_core::page::Frontmatter {
                title: "Test".into(),
                ..Default::default()
            },
            raw_content: "## Section A\n\nText\n\n## Section B\n\nMore text".to_string(),
            rendered_html: None,
            body_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }];

        render_markdown(&mut pages);

        assert_eq!(pages[0].toc.len(), 2);
        assert_eq!(pages[0].toc[0].text, "Section A");
        assert_eq!(pages[0].toc[1].text, "Section B");
        assert!(pages[0]
            .rendered_html
            .as_ref()
            .unwrap()
            .contains("id=\"section-a\""));
    }

    fn make_page(raw_content: &str) -> Page {
        Page {
            source_path: std::path::PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: mythic_core::page::Frontmatter {
                title: "Test".into(),
                ..Default::default()
            },
            raw_content: raw_content.to_string(),
            rendered_html: None,
            body_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn footnotes_render_correctly() {
        let md = "This has a footnote[^1].\n\n[^1]: The footnote content.";
        let html = render_one(md);
        assert!(html.contains("footnote"));
        assert!(html.contains("The footnote content"));
    }

    #[test]
    fn multiple_code_blocks_different_languages() {
        let highlighter = Highlighter::new("base16-ocean.dark", false);
        let md =
            "```rust\nfn main() {}\n```\n\nSome text\n\n```python\ndef hello():\n    pass\n```";
        let html = render_one_highlighted(md, &highlighter);
        // Both code blocks should contain highlighted spans
        assert!(html.contains("main"));
        assert!(html.contains("hello"));
        // Should have two <pre><code> blocks
        let pre_count = html.matches("<pre><code>").count();
        assert_eq!(pre_count, 2);
    }

    #[test]
    fn no_headings_produces_empty_toc() {
        let mut pages = vec![make_page("Just a paragraph.\n\nAnother paragraph.")];
        render_markdown(&mut pages);
        assert!(pages[0].toc.is_empty());
        assert!(pages[0].rendered_html.is_some());
    }

    #[test]
    fn only_h1_headings_filtered_by_default_toc() {
        // Default config has toc_min_level=2, so h1 headings should NOT appear in toc
        let mut pages = vec![make_page("# Title One\n\n# Title Two\n\nSome text")];
        render_markdown(&mut pages);
        assert!(pages[0].toc.is_empty());
        // But the headings should still be in the rendered HTML
        let html = pages[0].rendered_html.as_ref().unwrap();
        assert!(html.contains("Title One"));
        assert!(html.contains("Title Two"));
    }

    #[test]
    fn links_preserved_in_markdown() {
        let md = "Check out [Rust](https://www.rust-lang.org) and [Mythic](/about).";
        let html = render_one(md);
        assert!(html.contains("href=\"https://www.rust-lang.org\""));
        assert!(html.contains("href=\"/about\""));
        assert!(html.contains(">Rust<"));
        assert!(html.contains(">Mythic<"));
    }

    #[test]
    fn images_preserved_in_markdown() {
        let md = "![Alt text](image.png)\n\n![Logo](https://example.com/logo.svg \"Title\")";
        let html = render_one(md);
        assert!(html.contains("<img"));
        assert!(html.contains("src=\"image.png\""));
        assert!(html.contains("alt=\"Alt text\""));
        assert!(html.contains("src=\"https://example.com/logo.svg\""));
    }

    #[test]
    fn mixed_content_headings_code_lists_tables() {
        let highlighter = Highlighter::new("base16-ocean.dark", false);
        let md = "\
## Introduction

Here is a list:

- alpha
- beta

## Code Example

```rust
fn add(a: i32, b: i32) -> i32 { a + b }
```

## Data Table

| Name  | Value |
|-------|-------|
| x     | 42    |
";
        let html = render_one_highlighted(md, &highlighter);
        // Headings
        assert!(html.contains("Introduction"));
        assert!(html.contains("Code Example"));
        assert!(html.contains("Data Table"));
        // List
        assert!(html.contains("<li>alpha</li>"));
        assert!(html.contains("<li>beta</li>"));
        // Code
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("add"));
        // Table
        assert!(html.contains("<table>"));
        assert!(html.contains("42"));
    }

    #[test]
    fn render_markdown_with_config_custom_theme() {
        let mut pages = vec![make_page("```rust\nlet x = 1;\n```")];
        let config = RenderConfig {
            highlight_theme: "InspiredGitHub".to_string(),
            ..Default::default()
        };
        render_markdown_with_config(&mut pages, &config);
        let html = pages[0].rendered_html.as_ref().unwrap();
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("<span"));
    }

    #[test]
    fn render_markdown_with_config_line_numbers() {
        let mut pages = vec![make_page("```rust\nlet a = 1;\nlet b = 2;\n```")];
        let config = RenderConfig {
            line_numbers: true,
            ..Default::default()
        };
        render_markdown_with_config(&mut pages, &config);
        let html = pages[0].rendered_html.as_ref().unwrap();
        assert!(html.contains("line-number"));
    }

    #[test]
    fn parallel_rendering_determinism() {
        let md = "## Hello\n\nParagraph\n\n```rust\nfn f() {}\n```\n\n## World";
        let mut pages_a: Vec<Page> = (0..10).map(|_| make_page(md)).collect();
        let mut pages_b: Vec<Page> = (0..10).map(|_| make_page(md)).collect();

        render_markdown(&mut pages_a);
        render_markdown(&mut pages_b);

        for (a, b) in pages_a.iter().zip(pages_b.iter()) {
            assert_eq!(a.rendered_html, b.rendered_html);
            assert_eq!(a.toc.len(), b.toc.len());
            for (ta, tb) in a.toc.iter().zip(b.toc.iter()) {
                assert_eq!(ta.text, tb.text);
                assert_eq!(ta.id, tb.id);
                assert_eq!(ta.level, tb.level);
            }
        }
    }

    #[test]
    fn admonition_note_renders_as_div() {
        let md = "> [!NOTE]\n> This is a note";
        let html = render_one(md);
        assert!(html.contains("class=\"admonition admonition-note\""));
        assert!(html.contains("class=\"admonition-title\">Note</p>"));
        assert!(html.contains("This is a note"));
        // Should not contain blockquote
        assert!(!html.contains("<blockquote>"));
    }

    #[test]
    fn admonition_warning_gets_correct_class() {
        let md = "> [!WARNING]\n> Be careful";
        let html = render_one(md);
        assert!(html.contains("class=\"admonition admonition-warning\""));
        assert!(html.contains("class=\"admonition-title\">Warning</p>"));
        assert!(html.contains("Be careful"));
    }

    #[test]
    fn regular_blockquote_unchanged() {
        let md = "> Just a normal blockquote";
        let html = render_one(md);
        assert!(html.contains("<blockquote>"));
        assert!(!html.contains("admonition"));
    }

    #[test]
    fn multiple_admonitions_in_same_document() {
        let md = "> [!NOTE]\n> A note\n\n> [!TIP]\n> A tip\n\n> [!CAUTION]\n> Danger zone";
        let html = render_one(md);
        assert!(html.contains("admonition-note"));
        assert!(html.contains("admonition-tip"));
        assert!(html.contains("admonition-caution"));
        assert!(html.contains("class=\"admonition-title\">Note</p>"));
        assert!(html.contains("class=\"admonition-title\">Tip</p>"));
        assert!(html.contains("class=\"admonition-title\">Caution</p>"));
    }
}
