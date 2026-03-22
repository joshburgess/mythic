//! Search index generation for client-side search.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::page::Page;

/// A single entry in the search index.
#[derive(Debug, Serialize)]
struct SearchEntry {
    title: String,
    slug: String,
    url: String,
    summary: String,
    tags: Vec<String>,
}

/// Generate a JSON search index from all pages.
///
/// Writes `search-index.json` to the output directory containing
/// title, slug, URL, summary (first 200 chars), and tags for each page.
/// Suitable for client-side search libraries like Fuse.js or Lunr.js.
pub fn generate_search_index(pages: &[Page], output_dir: &Path, base_url: &str) -> Result<()> {
    let base_url = base_url.trim_end_matches('/');

    let entries: Vec<SearchEntry> = pages
        .iter()
        .filter(|p| !p.frontmatter.draft.unwrap_or(false))
        .filter(|p| !p.source_path.to_string_lossy().starts_with('<'))
        .map(|page| {
            let summary = page
                .rendered_html
                .as_deref()
                .or(Some(&page.raw_content))
                .map(|s| strip_html_and_truncate(s, 200))
                .unwrap_or_default();

            let tags = page
                .frontmatter
                .tags
                .as_ref()
                .map(|t| t.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();

            SearchEntry {
                title: page.frontmatter.title.to_string(),
                slug: page.slug.clone(),
                url: format!("{base_url}/{}/", page.slug),
                summary,
                tags,
            }
        })
        .collect();

    std::fs::create_dir_all(output_dir)?;
    let json = serde_json::to_string_pretty(&entries)?;
    std::fs::write(output_dir.join("search-index.json"), json)?;

    Ok(())
}

fn strip_html_and_truncate(html: &str, max_chars: usize) -> String {
    let mut text = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' {
            in_tag = false;
            continue;
        }
        if !in_tag {
            text.push(c);
            if text.len() >= max_chars {
                text.push_str("...");
                break;
            }
        }
    }

    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn test_page(title: &str, slug: &str, content: &str, tags: Vec<&str>) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: title.into(),
                tags: if tags.is_empty() {
                    None
                } else {
                    Some(tags.into_iter().map(|t| t.into()).collect())
                },
                ..Default::default()
            },
            raw_content: content.to_string(),
            rendered_html: Some(format!("<p>{content}</p>")),
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn generates_search_index() {
        let dir = tempfile::tempdir().unwrap();
        let pages = vec![
            test_page("Hello", "hello", "Hello world content", vec!["rust"]),
            test_page("About", "about", "About page", vec!["info"]),
        ];

        generate_search_index(&pages, dir.path(), "https://example.com").unwrap();

        let index_path = dir.path().join("search-index.json");
        assert!(index_path.exists());

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(index_path).unwrap()).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["title"], "Hello");
        assert_eq!(arr[0]["url"], "https://example.com/hello/");
        assert_eq!(arr[0]["tags"][0], "rust");
    }

    #[test]
    fn drafts_excluded_from_index() {
        let dir = tempfile::tempdir().unwrap();
        let mut draft = test_page("Draft", "draft", "Secret", vec![]);
        draft.frontmatter.draft = Some(true);

        let pages = vec![test_page("Public", "public", "Visible", vec![]), draft];

        generate_search_index(&pages, dir.path(), "https://example.com").unwrap();

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("search-index.json")).unwrap(),
        )
        .unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "Public");
    }

    #[test]
    fn summary_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let long_content = "word ".repeat(100);
        let pages = vec![test_page("Long", "long", &long_content, vec![])];

        generate_search_index(&pages, dir.path(), "https://example.com").unwrap();

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("search-index.json")).unwrap(),
        )
        .unwrap();
        let summary = json[0]["summary"].as_str().unwrap();
        assert!(summary.len() <= 210); // 200 + "..."
        assert!(summary.ends_with("..."));
    }
}
