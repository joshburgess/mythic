//! Atom feed generation.

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
        &config.base_url,
    );

    let feed_path = output_dir.join("feed.xml");
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(&feed_path, &feed_xml)?;

    // Per-taxonomy feeds
    for taxonomy in taxonomies {
        if !taxonomy.config.feed {
            continue;
        }

        for term in &taxonomy.terms {
            let term_pages: Vec<&Page> = pages
                .iter()
                .filter(|p| {
                    term.pages.iter().any(|tp| tp.slug == p.slug)
                })
                .collect();

            if term_pages.is_empty() {
                continue;
            }

            let mut sorted = term_pages;
            sorted.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));
            sorted.truncate(feed_config.entries);

            let feed_title = format!("{} — {}", feed_config.title, term.name);
            let feed_xml = render_atom_feed(
                &feed_title,
                &config.base_url,
                feed_config.author.as_deref().unwrap_or(&config.title),
                &sorted,
                &config.base_url,
            );

            let term_dir = output_dir
                .join(&taxonomy.config.slug)
                .join(&term.slug);
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
    _site_url: &str,
) -> String {
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
        "  <link href=\"{base_url}/feed.xml\" rel=\"self\"/>\n"
    ));
    xml.push_str(&format!("  <link href=\"{base_url}/\"/>\n"));
    xml.push_str(&format!("  <updated>{updated_rfc}</updated>\n"));
    xml.push_str(&format!(
        "  <id>{base_url}/</id>\n"
    ));
    xml.push_str("  <author>\n");
    xml.push_str(&format!("    <name>{}</name>\n", escape_xml(author)));
    xml.push_str("  </author>\n");

    for page in pages {
        let page_url = format!("{base_url}/{}/", page.slug);
        let date = page
            .frontmatter
            .date
            .as_deref()
            .unwrap_or("1970-01-01");
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
            "    <summary>{}</summary>\n",
            escape_xml(&summary)
        ));
        xml.push_str("  </entry>\n");
    }

    xml.push_str("</feed>\n");
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
    use crate::config::{FeedConfig, SiteConfig, TaxonomyConfig};
    use crate::page::{Frontmatter, Page};
    use crate::taxonomy::build_taxonomies;
    use std::path::PathBuf;

    fn page(title: &str, slug: &str, date: &str, tags: Vec<&str>) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: title.to_string(),
                date: Some(date.to_string()),
                tags: if tags.is_empty() {
                    None
                } else {
                    Some(tags.into_iter().map(String::from).collect())
                },
                ..Default::default()
            },
            raw_content: "Some content here".to_string(),
            rendered_html: Some("<p>Some content here</p>".to_string()),
            output_path: None,
            content_hash: 0,
        }
    }

    fn feed_config() -> SiteConfig {
        let mut config = SiteConfig::for_testing("Test Site", "http://example.com");
        config.feed = Some(FeedConfig {
            title: "Test Feed".to_string(),
            author: Some("Test Author".to_string()),
            entries: 20,
        });
        config.taxonomies.push(TaxonomyConfig {
            name: "tags".to_string(),
            slug: "tags".to_string(),
            feed: true,
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
        let pages = vec![
            page("Rust Post", "rust-post", "2024-02-01", vec!["rust"]),
        ];
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
            .map(|i| page(&format!("Post {i}"), &format!("p{i}"), &format!("2024-01-{:02}", i + 1), vec![]))
            .collect();
        let taxonomies = build_taxonomies(&config, &pages);

        generate_feeds(&config, &pages, &taxonomies, dir.path()).unwrap();

        let feed = std::fs::read_to_string(dir.path().join("feed.xml")).unwrap();
        let entry_count = feed.matches("<entry>").count();
        assert_eq!(entry_count, 2);
    }
}
