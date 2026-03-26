//! Atom and RSS 2.0 feed generation.

use crate::config::SiteConfig;
use crate::page::Page;
use crate::taxonomy::Taxonomy;
use anyhow::Result;
use std::path::Path;

/// Generate the site-wide Atom feed and per-taxonomy feeds.
pub fn generate_feeds(
    config: &SiteConfig,
    pages: &[Page],
    taxonomies: &[Taxonomy],
    output_dir: &Path,
) -> Result<()> {
    let feed_config = match &config.feed {
        Some(fc) => fc,
        None => return Ok(()),
    };

    // Site-wide feed
    let mut feed_pages: Vec<&Page> = pages
        .iter()
        .filter(|p| p.frontmatter.date.is_some())
        .collect();
    feed_pages.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));
    feed_pages.truncate(feed_config.entries);

    let feed_xml = render_atom_feed(
        &feed_config.title,
        &config.base_url,
        feed_config.author.as_deref().unwrap_or(&config.title),
        &feed_pages,
        "feed.xml",
    );

    let feed_path = output_dir.join("feed.xml");
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(&feed_path, &feed_xml)?;

    // Also generate RSS 2.0
    let rss_xml = render_rss_feed(
        &feed_config.title,
        &config.base_url,
        feed_config.author.as_deref().unwrap_or(&config.title),
        &feed_pages,
        "rss.xml",
    );
    std::fs::write(output_dir.join("rss.xml"), &rss_xml)?;

    // Also generate JSON Feed
    let json_feed = render_json_feed(
        &feed_config.title,
        &config.base_url,
        feed_config.author.as_deref().unwrap_or(&config.title),
        &feed_pages,
        "feed.json",
    );
    std::fs::write(output_dir.join("feed.json"), &json_feed)?;

    // Per-taxonomy feeds
    for taxonomy in taxonomies {
        if !taxonomy.config.feed {
            continue;
        }

        for term in &taxonomy.terms {
            let term_pages: Vec<&Page> = pages
                .iter()
                .filter(|p| term.pages.iter().any(|tp| tp.slug == p.slug))
                .collect();

            if term_pages.is_empty() {
                continue;
            }

            let mut sorted = term_pages;
            sorted.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));
            sorted.truncate(feed_config.entries);

            let feed_title = format!("{} — {}", feed_config.title, term.name);
            let term_feed_path = format!("{}/{}/feed.xml", taxonomy.config.slug, term.slug);
            let feed_xml = render_atom_feed(
                &feed_title,
                &config.base_url,
                feed_config.author.as_deref().unwrap_or(&config.title),
                &sorted,
                &term_feed_path,
            );

            let term_dir = output_dir.join(&taxonomy.config.slug).join(&term.slug);
            std::fs::create_dir_all(&term_dir)?;
            std::fs::write(term_dir.join("feed.xml"), &feed_xml)?;
        }
    }

    Ok(())
}

fn render_atom_feed(
    title: &str,
    base_url: &str,
    author: &str,
    pages: &[&Page],
    feed_path: &str,
) -> String {
    let base_url = base_url.trim_end_matches('/');

    let updated = pages
        .first()
        .and_then(|p| p.frontmatter.date.as_deref())
        .unwrap_or("1970-01-01");

    let updated_rfc = format!("{updated}T00:00:00Z");

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
    xml.push_str(&format!("  <title>{}</title>\n", escape_xml(title)));
    xml.push_str(&format!(
        "  <link href=\"{base_url}/{feed_path}\" rel=\"self\"/>\n"
    ));
    xml.push_str(&format!("  <link href=\"{base_url}/\"/>\n"));
    xml.push_str(&format!("  <updated>{updated_rfc}</updated>\n"));
    xml.push_str(&format!("  <id>{base_url}/</id>\n"));
    xml.push_str("  <author>\n");
    xml.push_str(&format!("    <name>{}</name>\n", escape_xml(author)));
    xml.push_str("  </author>\n");

    for page in pages {
        let page_url = format!("{base_url}/{}/", page.slug);
        let date = page.frontmatter.date.as_deref().unwrap_or("1970-01-01");
        let date_rfc = format!("{date}T00:00:00Z");

        let summary = page
            .rendered_html
            .as_deref()
            .or(Some(&page.raw_content))
            .map(|s| strip_html_and_truncate(s, 200))
            .unwrap_or_default();

        xml.push_str("  <entry>\n");
        xml.push_str(&format!(
            "    <title>{}</title>\n",
            escape_xml(&page.frontmatter.title)
        ));
        xml.push_str(&format!("    <link href=\"{page_url}\"/>\n"));
        xml.push_str(&format!("    <id>{page_url}</id>\n"));
        xml.push_str(&format!("    <updated>{date_rfc}</updated>\n"));
        xml.push_str(&format!("    <published>{date_rfc}</published>\n"));
        xml.push_str(&format!(
            "    <summary type=\"text\">{}</summary>\n",
            escape_xml(&summary)
        ));
        xml.push_str("  </entry>\n");
    }

    xml.push_str("</feed>\n");
    xml
}

fn render_rss_feed(title: &str, base_url: &str, author: &str, pages: &[&Page], feed_path: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    let pub_date = pages
        .first()
        .and_then(|p| p.frontmatter.date.as_deref())
        .unwrap_or("1970-01-01");

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n");
    xml.push_str("<channel>\n");
    xml.push_str(&format!("  <title>{}</title>\n", escape_xml(title)));
    xml.push_str(&format!("  <link>{base_url}/</link>\n"));
    xml.push_str(&format!(
        "  <atom:link href=\"{base_url}/{feed_path}\" rel=\"self\" type=\"application/rss+xml\"/>\n"
    ));
    xml.push_str(&format!("  <lastBuildDate>{}</lastBuildDate>\n", to_rfc822(pub_date)));
    xml.push_str(&format!(
        "  <managingEditor>{}</managingEditor>\n",
        escape_xml(author)
    ));

    for page in pages {
        let page_url = format!("{base_url}/{}/", page.slug);
        let date = page.frontmatter.date.as_deref().unwrap_or("1970-01-01");

        let summary = page
            .rendered_html
            .as_deref()
            .or(Some(&page.raw_content))
            .map(|s| strip_html_and_truncate(s, 200))
            .unwrap_or_default();

        xml.push_str("  <item>\n");
        xml.push_str(&format!(
            "    <title>{}</title>\n",
            escape_xml(&page.frontmatter.title)
        ));
        xml.push_str(&format!("    <link>{page_url}</link>\n"));
        xml.push_str(&format!("    <guid>{page_url}</guid>\n"));
        xml.push_str(&format!("    <pubDate>{}</pubDate>\n", to_rfc822(date)));
        xml.push_str(&format!(
            "    <description>{}</description>\n",
            escape_xml(&summary)
        ));
        xml.push_str("  </item>\n");
    }

    xml.push_str("</channel>\n");
    xml.push_str("</rss>\n");
    xml
}

fn render_json_feed(title: &str, base_url: &str, author: &str, pages: &[&Page], feed_path: &str) -> String {
    let base_url = base_url.trim_end_matches('/');

    let items: Vec<serde_json::Value> = pages
        .iter()
        .map(|page| {
            let page_url = format!("{base_url}/{}/", page.slug);
            let date = page.frontmatter.date.as_deref().unwrap_or("1970-01-01");
            let summary = page
                .rendered_html
                .as_deref()
                .or(Some(&page.raw_content))
                .map(|s| strip_html_and_truncate(s, 200))
                .unwrap_or_default();

            serde_json::json!({
                "id": page_url,
                "url": page_url,
                "title": page.frontmatter.title.as_str(),
                "content_text": summary,
                "date_published": format!("{date}T00:00:00Z"),
            })
        })
        .collect();

    let feed = serde_json::json!({
        "version": "https://jsonfeed.org/version/1.1",
        "title": title,
        "home_page_url": format!("{base_url}/"),
        "feed_url": format!("{base_url}/{feed_path}"),
        "authors": [{ "name": author }],
        "items": items,
    });

    serde_json::to_string_pretty(&feed).unwrap_or_default()
}

fn escape_xml(s: &str) -> String {
    strip_xml_invalid(s)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Strip characters that are invalid in XML 1.0.
/// Valid: #x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]
fn strip_xml_invalid(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            matches!(c,
                '\u{09}' | '\u{0A}' | '\u{0D}' |
                '\u{20}'..='\u{D7FF}' |
                '\u{E000}'..='\u{FFFD}' |
                '\u{10000}'..='\u{10FFFF}'
            )
        })
        .collect()
}

/// Convert an ISO date string (e.g. `2024-01-15`) to RFC 822 format
/// (`Tue, 15 Jan 2024 00:00:00 +0000`) as required by RSS 2.0.
fn to_rfc822(date_str: &str) -> String {
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        date.format("%a, %d %b %Y 00:00:00 +0000").to_string()
    } else {
        date_str.to_string()
    }
}

fn strip_html_and_truncate(html: &str, max_chars: usize) -> String {
    let mut text = String::new();
    let mut char_count = 0;
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
            char_count += 1;
            if char_count >= max_chars {
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
    use crate::config::{FeedConfig, SiteConfig, TaxonomyConfig};
    use crate::page::{Frontmatter, Page};
    use crate::taxonomy::build_taxonomies;
    use std::path::PathBuf;

    fn page(title: &str, slug: &str, date: &str, tags: Vec<&str>) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: title.into(),
                date: Some(date.into()),
                tags: if tags.is_empty() {
                    None
                } else {
                    Some(
                        tags.into_iter()
                            .map(compact_str::CompactString::from)
                            .collect(),
                    )
                },
                ..Default::default()
            },
            raw_content: "Some content here".to_string(),
            rendered_html: Some("<p>Some content here</p>".to_string()),
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    fn feed_config() -> SiteConfig {
        let mut config = SiteConfig::for_testing("Test Site", "http://example.com");
        config.feed = Some(FeedConfig {
            title: "Test Feed".into(),
            author: Some("Test Author".to_string()),
            entries: 20,
        });
        config.taxonomies.push(TaxonomyConfig {
            name: "tags".to_string(),
            slug: "tags".to_string(),
            feed: true,
            per_page: 10,
        });
        config
    }

    #[test]
    fn site_feed_generated() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![
            page("Post A", "a", "2024-02-01", vec!["rust"]),
            page("Post B", "b", "2024-01-15", vec!["web"]),
        ];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert!(feed.contains("<feed xmlns="));
        assert!(feed.contains("Test Feed"));
        assert!(feed.contains("Post A"));
        assert!(feed.contains("Post B"));
    }

    #[test]
    fn taxonomy_feed_generated() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![page("Rust Post", "rust-post", "2024-02-01", vec!["rust"])];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed_path = dir.path().join("tags/rust/feed.xml");
        assert!(feed_path.exists());
        let feed = std::fs::read_to_string(feed_path).unwrap();
        assert!(feed.contains("Rust Post"));
    }

    #[test]
    fn entry_limit_respected() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = feed_config();
        config.feed.as_mut().unwrap().entries = 2;

        let pages: Vec<Page> = (0..5)
            .map(|i| {
                page(
                    &format!("Post {i}"),
                    &format!("p{i}"),
                    &format!("2024-01-{:02}", i + 1),
                    vec![],
                )
            })
            .collect();
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        let entry_count = feed.matches("<entry>").count();
        assert_eq!(entry_count, 2);
    }

    #[test]
    fn feed_entries_sorted_by_date_descending() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![
            page("Old Post", "old", "2023-01-01", vec![]),
            page("New Post", "new", "2024-06-15", vec![]),
            page("Mid Post", "mid", "2024-03-10", vec![]),
        ];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        let new_pos = feed.find("New Post").unwrap();
        let mid_pos = feed.find("Mid Post").unwrap();
        let old_pos = feed.find("Old Post").unwrap();
        assert!(new_pos < mid_pos, "newest should appear first");
        assert!(mid_pos < old_pos, "middle date should appear second");
    }

    #[test]
    fn feed_with_no_dated_pages_produces_empty_entries() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        // Pages without dates
        let pages = vec![Page {
            source_path: PathBuf::from("nodates.md"),
            slug: "nodates".to_string(),
            frontmatter: Frontmatter {
                title: "No Date".into(),
                date: None,
                ..Default::default()
            },
            raw_content: "content".to_string(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert_eq!(feed.matches("<entry>").count(), 0);
    }

    #[test]
    fn xml_special_characters_in_titles_are_escaped() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![page(
            "Tom & Jerry <3 \"Quotes\"",
            "special",
            "2024-01-01",
            vec![],
        )];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert!(feed.contains("Tom &amp; Jerry &lt;3 &quot;Quotes&quot;"));
        // Should not contain unescaped ampersand in title context
        assert!(!feed.contains("<title>Tom & Jerry"));
    }

    #[test]
    fn feed_respects_base_url_in_links() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = feed_config();
        config.base_url = "https://mysite.org".to_string();
        let pages = vec![page("Post", "blog/hello", "2024-05-01", vec![])];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert!(feed.contains("https://mysite.org/blog/hello/"));
        assert!(feed.contains("https://mysite.org/feed.xml"));
        assert!(!feed.contains("http://example.com"));
    }

    #[test]
    fn feed_with_empty_title_config() {
        // Zola issue #2024: empty feed title should not produce invalid XML
        let dir = tempfile::tempdir().unwrap();
        let mut config = feed_config();
        config.feed.as_mut().unwrap().title = String::new();
        let pages = vec![page("Post", "p", "2024-01-01", vec![])];
        let taxonomies = build_taxonomies(&config, &pages);

        // Should still generate valid XML (even with empty title)
        let result = generate_feeds(&config, &pages, &taxonomies, dir.path());
        assert!(result.is_ok());
        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert!(feed.contains("<title>"));
    }

    // --- Hugo regression tests ---

    #[test]
    fn rss_feed_has_valid_xml_structure() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![
            page("Post A", "a", "2024-02-01", vec![]),
            page("Post B", "b", "2024-01-15", vec![]),
        ];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let rss = std::fs::read_to_string(dir.path().join("rss.xml")).unwrap();
        assert!(rss.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(rss.contains("<rss version=\"2.0\""));
        assert!(rss.contains("<channel>"));
        assert!(rss.contains("</channel>"));
        assert!(rss.contains("</rss>"));
        assert!(rss.contains("<item>"));
        assert!(rss.contains("</item>"));
        assert!(rss.contains("Post A"));
        assert!(rss.contains("Post B"));
    }

    #[test]
    fn json_feed_has_valid_json_and_correct_version() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![
            page("Post A", "a", "2024-02-01", vec![]),
            page("Post B", "b", "2024-01-15", vec![]),
        ];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let json_str = std::fs::read_to_string(dir.path().join("feed.json")).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("feed.json must be valid JSON");
        assert_eq!(
            parsed["version"].as_str().unwrap(),
            "https://jsonfeed.org/version/1.1",
            "JSON Feed version must be 1.1"
        );
        assert_eq!(parsed["title"].as_str().unwrap(), "Test Feed");
    }

    #[test]
    fn json_feed_has_correct_item_count() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![
            page("Post A", "a", "2024-03-01", vec![]),
            page("Post B", "b", "2024-02-01", vec![]),
            page("Post C", "c", "2024-01-01", vec![]),
        ];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let json_str = std::fs::read_to_string(dir.path().join("feed.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let items = parsed["items"].as_array().expect("items must be an array");
        assert_eq!(items.len(), 3, "JSON Feed should contain exactly 3 items");
    }

    #[test]
    fn rss_and_json_feeds_respect_entry_limit() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = feed_config();
        config.feed.as_mut().unwrap().entries = 3;

        let pages: Vec<Page> = (0..7)
            .map(|i| {
                page(
                    &format!("Post {i}"),
                    &format!("p{i}"),
                    &format!("2024-01-{:02}", i + 1),
                    vec![],
                )
            })
            .collect();
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        // RSS feed should have at most 3 items
        let rss = std::fs::read_to_string(dir.path().join("rss.xml")).unwrap();
        let rss_item_count = rss.matches("<item>").count();
        assert_eq!(
            rss_item_count, 3,
            "RSS feed should respect entry limit of 3"
        );

        // JSON feed should have at most 3 items
        let json_str = std::fs::read_to_string(dir.path().join("feed.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let json_item_count = parsed["items"].as_array().unwrap().len();
        assert_eq!(
            json_item_count, 3,
            "JSON Feed should respect entry limit of 3"
        );
    }

    #[test]
    fn feed_strips_control_characters() {
        // Hugo #3268: XML control characters in content must be stripped.
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let mut p = page("Post with control", "ctrl", "2024-01-01", vec![]);
        p.rendered_html = Some("Text with \x0B vertical tab and \x00 null".to_string());
        let pages = vec![p];
        let taxonomies = build_taxonomies(&config, &pages);

        let result = generate_feeds(&config, &pages, &taxonomies, dir.path());
        assert!(result.is_ok());
        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert!(feed.contains("Post with control"));
        // Control characters must be stripped
        assert!(!feed.contains('\x0B'), "Vertical tab should be stripped");
        assert!(!feed.contains('\x00'), "Null byte should be stripped");
    }

    #[test]
    fn atom_self_link_no_double_slashes_with_trailing_slash_base_url() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = feed_config();
        // base_url with trailing slash
        config.base_url = "https://example.com/".to_string();
        let pages = vec![page("Post", "hello", "2024-01-01", vec![])];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        // The self-link should not have double slashes after the domain
        assert!(
            !feed.contains("example.com//"),
            "Feed should not have double slashes in URLs, got: {feed}"
        );
        assert!(feed.contains("example.com/feed.xml"));
    }

    #[test]
    fn atom_summary_has_type_text_attribute() {
        let dir = tempfile::tempdir().unwrap();
        let config = feed_config();
        let pages = vec![page("Post", "hello", "2024-01-01", vec![])];
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        assert!(
            feed.contains("type=\"text\""),
            "Atom <summary> should have type=\"text\" attribute, got: {feed}"
        );
    }
}
