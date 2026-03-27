//! Schema.org JSON-LD structured data auto-generation.
//!
//! Generates JSON-LD `<script>` tags from page frontmatter for SEO.
//! Supports Article, BlogPosting, BreadcrumbList, and WebPage types.

use crate::page::Page;

/// Generate a JSON-LD `<script>` tag for a page's structured data.
///
/// The schema type is inferred from the page's layout and content:
/// - `blog` / `post` layout → BlogPosting
/// - Pages with a date → Article
/// - All others → WebPage
pub fn generate_jsonld(page: &Page, site_title: &str, base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    let page_url = format!("{base_url}/{}/", page.slug);

    let layout = page.frontmatter.layout.as_deref().unwrap_or("default");
    let schema_type = match layout {
        "blog" | "post" => "BlogPosting",
        _ if page.frontmatter.date.is_some() => "Article",
        _ => "WebPage",
    };

    let mut ld = serde_json::json!({
        "@context": "https://schema.org",
        "@type": schema_type,
        "headline": page.frontmatter.title.as_str(),
        "url": page_url,
        "isPartOf": {
            "@type": "WebSite",
            "name": site_title,
            "url": format!("{base_url}/"),
        },
    });

    if let Some(ref date) = page.frontmatter.date {
        ld["datePublished"] = serde_json::Value::String(date.to_string());
        ld["dateModified"] = serde_json::Value::String(date.to_string());
    }

    if let Some(ref tags) = page.frontmatter.tags {
        let keywords: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
        ld["keywords"] = serde_json::Value::String(keywords.join(", "));
    }

    // Add author if present in extra
    if let Some(ref extra) = page.frontmatter.extra {
        if let Some(author) = extra.get("author").and_then(|v| v.as_str()) {
            ld["author"] = serde_json::json!({
                "@type": "Person",
                "name": author,
            });
        }

        if let Some(description) = extra.get("description").and_then(|v| v.as_str()) {
            ld["description"] = serde_json::Value::String(description.to_string());
        }

        if let Some(summary) = extra.get("summary").and_then(|v| v.as_str()) {
            if ld.get("description").is_none() {
                ld["description"] = serde_json::Value::String(summary.to_string());
            }
        }
    }

    // Add word count if available
    let word_count = page.raw_content.split_whitespace().count();
    if word_count > 0 {
        ld["wordCount"] = serde_json::Value::Number(serde_json::Number::from(word_count));
    }

    let json = serde_json::to_string(&ld).unwrap_or_default();
    format!("<script type=\"application/ld+json\">{json}</script>")
}

/// Generate BreadcrumbList JSON-LD for a page based on its slug path.
pub fn generate_breadcrumbs(page: &Page, site_title: &str, base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    let parts: Vec<&str> = page.slug.split('/').collect();

    if parts.len() <= 1 {
        return String::new();
    }

    let mut items = Vec::new();

    // Home
    items.push(serde_json::json!({
        "@type": "ListItem",
        "position": 1,
        "name": site_title,
        "item": format!("{base_url}/"),
    }));

    // Intermediate segments
    let mut path = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            path.push('/');
        }
        path.push_str(part);

        let name = if i == parts.len() - 1 {
            page.frontmatter.title.to_string()
        } else {
            // Capitalize the directory name
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.collect::<String>()),
                None => part.to_string(),
            }
        };

        items.push(serde_json::json!({
            "@type": "ListItem",
            "position": i + 2,
            "name": name,
            "item": format!("{base_url}/{path}/"),
        }));
    }

    let ld = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": items,
    });

    let json = serde_json::to_string(&ld).unwrap_or_default();
    format!("<script type=\"application/ld+json\">{json}</script>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn test_page(slug: &str, layout: &str) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: "Test Post".into(),
                date: Some("2024-06-15".into()),
                layout: Some(layout.into()),
                tags: Some(vec!["rust".into(), "web".into()]),
                ..Default::default()
            },
            raw_content: "Some content here".to_string(),
            rendered_html: None,
            body_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn blog_post_generates_blog_posting_schema() {
        let page = test_page("blog/my-post", "blog");
        let ld = generate_jsonld(&page, "My Site", "https://example.com");
        assert!(ld.contains("application/ld+json"));
        assert!(ld.contains("BlogPosting"));
        assert!(ld.contains("Test Post"));
        assert!(ld.contains("2024-06-15"));
        assert!(ld.contains("rust, web"));
    }

    #[test]
    fn dated_page_generates_article_schema() {
        let page = test_page("docs/guide", "default");
        let ld = generate_jsonld(&page, "My Site", "https://example.com");
        assert!(ld.contains("Article"));
    }

    #[test]
    fn undated_page_generates_webpage_schema() {
        let mut page = test_page("about", "default");
        page.frontmatter.date = None;
        let ld = generate_jsonld(&page, "My Site", "https://example.com");
        assert!(ld.contains("WebPage"));
    }

    #[test]
    fn breadcrumbs_for_nested_page() {
        let page = test_page("blog/2024/my-post", "post");
        let bc = generate_breadcrumbs(&page, "My Site", "https://example.com");
        assert!(bc.contains("BreadcrumbList"));
        assert!(bc.contains("My Site"));
        assert!(bc.contains("Blog"));
        assert!(bc.contains("Test Post"));
        assert!(bc.contains("\"position\":1"));
    }

    #[test]
    fn breadcrumbs_empty_for_root_page() {
        let page = test_page("about", "default");
        let bc = generate_breadcrumbs(&page, "My Site", "https://example.com");
        assert!(bc.is_empty());
    }

    #[test]
    fn author_included_when_in_extra() {
        let mut page = test_page("post", "blog");
        let extra = page.frontmatter.extra.get_or_insert_with(Default::default);
        extra.insert(
            "author".to_string(),
            serde_json::Value::String("Jane Doe".to_string()),
        );
        let ld = generate_jsonld(&page, "My Site", "https://example.com");
        assert!(ld.contains("Jane Doe"));
        assert!(ld.contains("Person"));
    }
}
