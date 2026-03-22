//! Content summary extraction using the `<!--more-->` marker.
//!
//! If a page's raw content contains `<!--more-->`, the text before it
//! becomes the summary. The full content is still rendered normally.

use crate::page::Page;

/// The summary split marker.
const MORE_MARKER: &str = "<!--more-->";

/// Extract summary from raw content and store it in `page.extra["summary"]`.
///
/// If the marker is present, the text before it becomes the summary.
/// If not present, the first 200 characters of stripped HTML are used.
pub fn extract_summaries(pages: &mut [Page]) {
    for page in pages.iter_mut() {
        let summary = if let Some(pos) = page.raw_content.find(MORE_MARKER) {
            page.raw_content[..pos].trim().to_string()
        } else {
            // Auto-summary: first 200 chars of raw content
            let mut summary = page.raw_content.chars().take(200).collect::<String>();
            if page.raw_content.len() > 200 {
                summary.push_str("...");
            }
            summary
        };

        let extra = page
            .frontmatter
            .extra
            .get_or_insert_with(std::collections::HashMap::new);
        extra.insert("summary".to_string(), serde_json::Value::String(summary));
        extra.insert(
            "truncated".to_string(),
            serde_json::Value::Bool(page.raw_content.contains(MORE_MARKER)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn make_page(content: &str) -> Page {
        Page {
            source_path: PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: Frontmatter {
                title: "Test".into(),
                ..Default::default()
            },
            raw_content: content.to_string(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn extracts_summary_from_more_marker() {
        let mut pages = vec![make_page(
            "This is the intro.\n\n<!--more-->\n\nThis is the full content.",
        )];
        extract_summaries(&mut pages);

        let extra = pages[0].frontmatter.extra.as_ref().unwrap();
        assert_eq!(extra["summary"], "This is the intro.");
        assert_eq!(extra["truncated"], true);
    }

    #[test]
    fn auto_summary_without_marker() {
        let mut pages = vec![make_page("Short content without a marker.")];
        extract_summaries(&mut pages);

        let extra = pages[0].frontmatter.extra.as_ref().unwrap();
        assert_eq!(extra["summary"], "Short content without a marker.");
        assert_eq!(extra["truncated"], false);
    }

    #[test]
    fn auto_summary_truncates_long_content() {
        let long = "word ".repeat(100); // 500 chars
        let mut pages = vec![make_page(&long)];
        extract_summaries(&mut pages);

        let extra = pages[0].frontmatter.extra.as_ref().unwrap();
        let summary = extra["summary"].as_str().unwrap();
        assert!(summary.ends_with("..."));
        assert!(summary.len() <= 210);
    }

    #[test]
    fn marker_at_start() {
        let mut pages = vec![make_page("<!--more-->\n\nAll content after marker.")];
        extract_summaries(&mut pages);

        let extra = pages[0].frontmatter.extra.as_ref().unwrap();
        assert_eq!(extra["summary"], "");
        assert_eq!(extra["truncated"], true);
    }
}
