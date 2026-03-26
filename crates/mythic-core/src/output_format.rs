//! Custom output formats for rendering pages in multiple formats.
//!
//! In addition to the default HTML output, pages can be rendered as
//! JSON (for API-like consumption) or plain text.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::page::Page;

/// An output format definition.
#[derive(Debug, Clone)]
pub enum OutputFormat {
    /// Standard HTML output (default).
    Html,
    /// JSON representation of page data.
    Json,
    /// Plain text (stripped HTML).
    PlainText,
}

/// JSON representation of a page for API consumption.
#[derive(Debug, Serialize)]
struct PageJson {
    title: String,
    slug: String,
    url: String,
    date: Option<String>,
    tags: Vec<String>,
    content: String,
    summary: Option<String>,
    word_count: usize,
}

/// Generate JSON output for a page.
pub fn render_json(page: &Page, base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    let content = page.rendered_html.as_deref().unwrap_or(&page.raw_content);

    let summary = page
        .frontmatter
        .extra
        .as_ref()
        .and_then(|e| e.get("summary"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let tags = page
        .frontmatter
        .tags
        .as_ref()
        .map(|t| t.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let url = if page.slug == "index" {
        format!("{base_url}/")
    } else {
        format!("{base_url}/{}/", page.slug)
    };

    let json = PageJson {
        title: page.frontmatter.title.to_string(),
        slug: page.slug.clone(),
        url,
        date: page.frontmatter.date.as_ref().map(|d| d.to_string()),
        tags,
        content: content.to_string(),
        summary,
        word_count: page.raw_content.split_whitespace().count(),
    };

    serde_json::to_string_pretty(&json).unwrap_or_default()
}

/// Generate JSON output files for all pages that have `json: true` in frontmatter
/// or when `json_api` is enabled in config.
pub fn generate_json_api(pages: &[Page], output_dir: &Path, base_url: &str) -> Result<usize> {
    let mut count = 0;

    for page in pages {
        // Check if page opts into JSON output via extra.json
        let wants_json = page
            .frontmatter
            .extra
            .as_ref()
            .and_then(|e| e.get("json"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !wants_json {
            continue;
        }

        let json = render_json(page, base_url);
        let dest = output_dir.join(&page.slug).join("index.json");
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, json)?;
        count += 1;
    }

    Ok(count)
}

/// Generate a full site JSON API index.
pub fn generate_api_index(pages: &[Page], output_dir: &Path, base_url: &str) -> Result<()> {
    let base_url = base_url.trim_end_matches('/');

    let entries: Vec<serde_json::Value> = pages
        .iter()
        .filter(|p| !p.frontmatter.draft.unwrap_or(false))
        .map(|p| {
            serde_json::json!({
                "title": p.frontmatter.title.as_str(),
                "slug": &p.slug,
                "url": if p.slug == "index" {
                    format!("{base_url}/")
                } else {
                    format!("{base_url}/{}/", p.slug)
                },
                "date": p.frontmatter.date.as_deref(),
            })
        })
        .collect();

    let api = serde_json::json!({
        "pages": entries,
        "total": entries.len(),
    });

    std::fs::create_dir_all(output_dir.join("api"))?;
    std::fs::write(
        output_dir.join("api/pages.json"),
        serde_json::to_string_pretty(&api)?,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn test_page(slug: &str, title: &str) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: title.into(),
                date: Some("2024-01-15".into()),
                tags: Some(vec!["rust".into(), "web".into()]),
                ..Default::default()
            },
            raw_content: "Hello world content here".to_string(),
            rendered_html: Some("<p>Hello world content here</p>".to_string()),
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn render_json_produces_valid_json() {
        let page = test_page("blog/post", "My Post");
        let json_str = render_json(&page, "https://example.com");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["title"], "My Post");
        assert_eq!(parsed["slug"], "blog/post");
        assert_eq!(parsed["url"], "https://example.com/blog/post/");
        assert_eq!(parsed["tags"][0], "rust");
        assert!(parsed["word_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn json_api_only_for_opted_in_pages() {
        let dir = tempfile::tempdir().unwrap();
        let mut page = test_page("post", "Post");
        // Not opted in — no JSON generated
        let count = generate_json_api(&[page.clone()], dir.path(), "https://example.com").unwrap();
        assert_eq!(count, 0);

        // Opt in via extra.json = true
        let extra = page.frontmatter.extra.get_or_insert_with(Default::default);
        extra.insert("json".to_string(), serde_json::Value::Bool(true));
        let count = generate_json_api(&[page], dir.path(), "https://example.com").unwrap();
        assert_eq!(count, 1);
        assert!(dir.path().join("post/index.json").exists());
    }

    #[test]
    fn api_index_generated() {
        let dir = tempfile::tempdir().unwrap();
        let pages = vec![test_page("a", "Post A"), test_page("b", "Post B")];
        generate_api_index(&pages, dir.path(), "https://example.com").unwrap();

        let index = std::fs::read_to_string(dir.path().join("api/pages.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&index).unwrap();
        assert_eq!(parsed["total"], 2);
        assert_eq!(parsed["pages"][0]["title"], "Post A");
    }
}
