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
    html_output
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
                        format!(
                            "<pre><code>{}</code></pre>",
                            escape_html(&code_content)
                        )
                        .into(),
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
    html_output
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
        let html = render_one("# Hello\n\nA **bold** paragraph with *italics*.\n\n- item 1\n- item 2");
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
                title: "Test".to_string(),
                ..Default::default()
            },
            raw_content: "## Section A\n\nText\n\n## Section B\n\nMore text".to_string(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }];

        render_markdown(&mut pages);

        assert_eq!(pages[0].toc.len(), 2);
        assert_eq!(pages[0].toc[0].text, "Section A");
        assert_eq!(pages[0].toc[1].text, "Section B");
        assert!(pages[0].rendered_html.as_ref().unwrap().contains("id=\"section-a\""));
    }
}
