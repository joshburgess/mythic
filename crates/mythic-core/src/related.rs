//! Related content engine — finds pages that share tags with a given page.

use crate::page::Page;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A reference to a related page, with a relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedPage {
    /// Title of the related page.
    pub title: String,
    /// URL slug of the related page.
    pub slug: String,
    /// Canonical URL path (e.g. `/blog/my-post/`).
    pub url: String,
    /// Number of shared tags (higher = more relevant).
    pub score: usize,
}

/// Find pages related to the given page by shared tags.
///
/// Returns up to `limit` related pages, sorted by relevance score (most shared
/// tags first). Pages with no tags, or pages that share zero tags with the
/// target page, are excluded.
pub fn find_related(page: &Page, all_pages: &[Page], limit: usize, base_path: &str) -> Vec<RelatedPage> {
    let page_tags: HashSet<String> = page
        .frontmatter
        .tags
        .as_ref()
        .map(|t| t.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    if page_tags.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(usize, &Page)> = all_pages
        .iter()
        .filter(|p| p.slug != page.slug)
        .filter_map(|p| {
            let other_tags: HashSet<String> = p
                .frontmatter
                .tags
                .as_ref()
                .map(|t| t.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            let shared = page_tags.intersection(&other_tags).count();
            if shared > 0 {
                Some((shared, p))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.truncate(limit);

    scored
        .into_iter()
        .map(|(score, p)| RelatedPage {
            title: p.frontmatter.title.to_string(),
            slug: p.slug.clone(),
            url: if p.slug == "index" {
                format!("{}/", base_path)
            } else {
                format!("{}/{}/", base_path, p.slug)
            },
            score,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use compact_str::CompactString;
    use std::path::PathBuf;

    fn make_page(slug: &str, title: &str, tags: &[&str]) -> Page {
        Page {
            source_path: PathBuf::from(format!("{}.md", slug)),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: CompactString::new(title),
                tags: if tags.is_empty() {
                    None
                } else {
                    Some(tags.iter().map(|t| CompactString::new(t)).collect())
                },
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn pages_with_shared_tags_are_found() {
        let target = make_page("a", "Page A", &["rust", "web"]);
        let all = vec![
            make_page("a", "Page A", &["rust", "web"]),
            make_page("b", "Page B", &["rust"]),
            make_page("c", "Page C", &["python"]),
        ];
        let related = find_related(&target, &all, 10, "");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].slug, "b");
        assert_eq!(related[0].score, 1);
    }

    #[test]
    fn pages_with_no_shared_tags_return_empty() {
        let target = make_page("a", "Page A", &["rust"]);
        let all = vec![
            make_page("a", "Page A", &["rust"]),
            make_page("b", "Page B", &["python"]),
            make_page("c", "Page C", &["go"]),
        ];
        let related = find_related(&target, &all, 10, "");
        assert!(related.is_empty());
    }

    #[test]
    fn results_sorted_by_score_desc() {
        let target = make_page("a", "Page A", &["rust", "web", "ssg"]);
        let all = vec![
            make_page("a", "Page A", &["rust", "web", "ssg"]),
            make_page("b", "Page B", &["rust"]),        // score 1
            make_page("c", "Page C", &["rust", "web"]), // score 2
            make_page("d", "Page D", &["rust", "web", "ssg"]), // score 3
        ];
        let related = find_related(&target, &all, 10, "");
        assert_eq!(related.len(), 3);
        assert_eq!(related[0].slug, "d");
        assert_eq!(related[0].score, 3);
        assert_eq!(related[1].slug, "c");
        assert_eq!(related[1].score, 2);
        assert_eq!(related[2].slug, "b");
        assert_eq!(related[2].score, 1);
    }

    #[test]
    fn limit_is_respected() {
        let target = make_page("a", "Page A", &["rust", "web"]);
        let all = vec![
            make_page("a", "Page A", &["rust", "web"]),
            make_page("b", "Page B", &["rust"]),
            make_page("c", "Page C", &["web"]),
            make_page("d", "Page D", &["rust", "web"]),
        ];
        let related = find_related(&target, &all, 2, "");
        assert_eq!(related.len(), 2);
    }

    #[test]
    fn page_does_not_include_itself() {
        let target = make_page("a", "Page A", &["rust"]);
        let all = vec![
            make_page("a", "Page A", &["rust"]),
            make_page("b", "Page B", &["rust"]),
        ];
        let related = find_related(&target, &all, 10, "");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].slug, "b");
    }

    #[test]
    fn page_with_no_tags_returns_empty() {
        let target = make_page("a", "Page A", &[]);
        let all = vec![
            make_page("a", "Page A", &[]),
            make_page("b", "Page B", &["rust"]),
        ];
        let related = find_related(&target, &all, 10, "");
        assert!(related.is_empty());
    }

    #[test]
    fn index_page_gets_root_url() {
        let target = make_page("a", "Page A", &["rust"]);
        let all = vec![
            make_page("a", "Page A", &["rust"]),
            make_page("index", "Home", &["rust"]),
            make_page("b", "Page B", &["rust"]),
        ];
        let related = find_related(&target, &all, 10, "");
        let index_related = related.iter().find(|r| r.slug == "index").unwrap();
        assert_eq!(index_related.url, "/", "index page URL should be root, not /index/");
        let b_related = related.iter().find(|r| r.slug == "b").unwrap();
        assert_eq!(b_related.url, "/b/", "non-index pages should have normal URL");
    }
}
