//! Pagination for taxonomy term pages and section listings.

use serde::{Deserialize, Serialize};

use crate::page::Page;

/// Pagination context available in templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginator {
    /// Pages for the current page of results.
    pub pages: Vec<PaginatorPage>,
    /// Current page number (1-based).
    pub current_page: usize,
    /// Total number of pages.
    pub total_pages: usize,
    /// Total number of items across all pages.
    pub total_items: usize,
    /// URL of the previous page (None if on first page).
    pub prev_url: Option<String>,
    /// URL of the next page (None if on last page).
    pub next_url: Option<String>,
    /// Items per page.
    pub per_page: usize,
}

/// Lightweight page reference for paginator context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatorPage {
    pub title: String,
    pub slug: String,
    pub date: Option<String>,
    pub url: String,
}

/// Generate paginated page sets from a list of pages.
///
/// Returns a Vec of (page_number, Paginator) tuples.
/// Page numbers are 1-based.
pub fn paginate(
    pages: &[Page],
    per_page: usize,
    base_slug: &str,
    base_url: &str,
) -> Vec<(usize, Paginator)> {
    if per_page == 0 || pages.is_empty() {
        return Vec::new();
    }

    let base_url = base_url.trim_end_matches('/');
    let total_items = pages.len();
    let total_pages = total_items.div_ceil(per_page);

    let mut result = Vec::with_capacity(total_pages);

    for page_num in 1..=total_pages {
        let start = (page_num - 1) * per_page;
        let end = (start + per_page).min(total_items);

        let page_items: Vec<PaginatorPage> = pages[start..end]
            .iter()
            .map(|p| PaginatorPage {
                title: p.frontmatter.title.to_string(),
                slug: p.slug.clone(),
                date: p.frontmatter.date.as_ref().map(|d| d.to_string()),
                url: format!("{base_url}/{}/", p.slug),
            })
            .collect();

        let prev_url = if page_num > 1 {
            if page_num == 2 {
                Some(format!("{base_url}/{base_slug}/"))
            } else {
                Some(format!("{base_url}/{base_slug}/page/{}/", page_num - 1))
            }
        } else {
            None
        };

        let next_url = if page_num < total_pages {
            Some(format!("{base_url}/{base_slug}/page/{}/", page_num + 1))
        } else {
            None
        };

        result.push((
            page_num,
            Paginator {
                pages: page_items,
                current_page: page_num,
                total_pages,
                total_items,
                prev_url,
                next_url,
                per_page,
            },
        ));
    }

    result
}

/// Generate the output slug for a paginated page.
pub fn paginated_slug(base_slug: &str, page_num: usize) -> String {
    if page_num == 1 {
        base_slug.to_string()
    } else {
        format!("{base_slug}/page/{page_num}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn make_pages(count: usize) -> Vec<Page> {
        (0..count)
            .map(|i| Page {
                source_path: PathBuf::from(format!("post-{i}.md")),
                slug: format!("post-{i}"),
                frontmatter: Frontmatter {
                    title: format!("Post {i}").into(),
                    date: Some(format!("2024-01-{:02}", (i % 28) + 1).into()),
                    ..Default::default()
                },
                raw_content: String::new(),
                rendered_html: None,
                body_html: None,
                output_path: None,
                content_hash: 0,
                toc: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn basic_pagination() {
        let pages = make_pages(25);
        let result = paginate(&pages, 10, "blog", "https://example.com");

        assert_eq!(result.len(), 3);

        let (num, p1) = &result[0];
        assert_eq!(*num, 1);
        assert_eq!(p1.pages.len(), 10);
        assert_eq!(p1.current_page, 1);
        assert_eq!(p1.total_pages, 3);
        assert_eq!(p1.total_items, 25);
        assert!(p1.prev_url.is_none());
        assert_eq!(
            p1.next_url.as_deref(),
            Some("https://example.com/blog/page/2/")
        );

        let (_, p2) = &result[1];
        assert_eq!(p2.pages.len(), 10);
        assert_eq!(p2.prev_url.as_deref(), Some("https://example.com/blog/"));
        assert_eq!(
            p2.next_url.as_deref(),
            Some("https://example.com/blog/page/3/")
        );

        let (_, p3) = &result[2];
        assert_eq!(p3.pages.len(), 5);
        assert!(p3.next_url.is_none());
    }

    #[test]
    fn single_page_no_pagination() {
        let pages = make_pages(5);
        let result = paginate(&pages, 10, "blog", "https://example.com");

        assert_eq!(result.len(), 1);
        let (_, p) = &result[0];
        assert_eq!(p.pages.len(), 5);
        assert!(p.prev_url.is_none());
        assert!(p.next_url.is_none());
    }

    #[test]
    fn empty_pages() {
        let result = paginate(&[], 10, "blog", "https://example.com");
        assert!(result.is_empty());
    }

    #[test]
    fn exact_page_boundary() {
        let pages = make_pages(20);
        let result = paginate(&pages, 10, "blog", "https://example.com");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1.pages.len(), 10);
        assert_eq!(result[1].1.pages.len(), 10);
    }

    #[test]
    fn paginated_slug_generation() {
        assert_eq!(paginated_slug("tags/rust", 1), "tags/rust");
        assert_eq!(paginated_slug("tags/rust", 2), "tags/rust/page/2");
        assert_eq!(paginated_slug("tags/rust", 10), "tags/rust/page/10");
    }

    #[test]
    fn per_page_one() {
        let pages = make_pages(3);
        let result = paginate(&pages, 1, "blog", "https://example.com");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].1.pages.len(), 1);
        assert_eq!(result[1].1.pages.len(), 1);
        assert_eq!(result[2].1.pages.len(), 1);
    }

    #[test]
    fn page_urls_use_base_url() {
        let pages = make_pages(5);
        let result = paginate(&pages, 10, "blog", "https://mysite.org");
        assert_eq!(result[0].1.pages[0].url, "https://mysite.org/post-0/");
    }
}
