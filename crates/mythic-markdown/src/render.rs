//! Markdown-to-HTML rendering using pulldown-cmark with GFM extensions.

use mythic_core::page::Page;
use pulldown_cmark::{html, Options, Parser};
use rayon::prelude::*;

/// Render markdown to HTML for all pages in parallel.
pub fn render_markdown(pages: &mut [Page]) {
    pages.par_iter_mut().for_each(|page| {
        page.rendered_html = Some(render_one(&page.raw_content));
    });
}

/// Render a single markdown string to HTML.
pub fn render_one(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, opts);
    let mut html_output = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut html_output, parser);
    html_output
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
    fn code_block() {
        let md = "```rust\nfn main() {}\n```";
        let html = render_one(md);
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }
}
