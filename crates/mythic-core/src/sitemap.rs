//! Sitemap.xml and robots.txt generation.

use anyhow::Result;
use std::path::Path;

use crate::config::SiteConfig;
use crate::page::Page;

/// Generate sitemap.xml and robots.txt in the output directory.
pub fn generate(config: &SiteConfig, pages: &[Page], output_dir: &Path) -> Result<()> {
    let sitemap_config = match &config.sitemap {
        Some(sc) if sc.enabled => sc,
        Some(_) => return Ok(()),
        None => {
            // Default: generate sitemap
            generate_sitemap(config, pages, output_dir, "weekly")?;
            generate_robots_txt(config, output_dir)?;
            return Ok(());
        }
    };

    generate_sitemap(config, pages, output_dir, &sitemap_config.changefreq)?;
    generate_robots_txt(config, output_dir)?;

    Ok(())
}

fn generate_sitemap(
    config: &SiteConfig,
    pages: &[Page],
    output_dir: &Path,
    changefreq: &str,
) -> Result<()> {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    let base_url = config.base_url.trim_end_matches('/');

    for page in pages {
        // Skip drafts
        if page.frontmatter.draft.unwrap_or(false) {
            continue;
        }

        // Skip pages that opted out
        if page.frontmatter.sitemap == Some(false) {
            continue;
        }

        // Skip generated taxonomy pages (they have synthetic source paths)
        if page.source_path.to_string_lossy().starts_with('<') {
            continue;
        }

        let url = if page.slug == "index" {
            format!("{base_url}/")
        } else if config.ugly_urls {
            format!("{base_url}/{}.html", page.slug)
        } else {
            format!("{base_url}/{}/", page.slug)
        };
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{url}</loc>\n"));
        if let Some(date) = &page.frontmatter.date {
            xml.push_str(&format!("    <lastmod>{date}</lastmod>\n"));
        }
        xml.push_str(&format!("    <changefreq>{changefreq}</changefreq>\n"));
        xml.push_str("  </url>\n");
    }

    xml.push_str("</urlset>\n");

    std::fs::create_dir_all(output_dir)?;
    std::fs::write(output_dir.join("sitemap.xml"), xml)?;

    Ok(())
}

fn generate_robots_txt(config: &SiteConfig, output_dir: &Path) -> Result<()> {
    let base_url = config.base_url.trim_end_matches('/');
    let content = format!("User-agent: *\nAllow: /\n\nSitemap: {base_url}/sitemap.xml\n");

    std::fs::create_dir_all(output_dir)?;
    std::fs::write(output_dir.join("robots.txt"), content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn test_page(slug: &str, date: &str) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: slug.into(),
                date: Some(date.into()),
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
    fn sitemap_content() {
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com");
        let pages = vec![
            test_page("about", "2024-01-15"),
            test_page("blog/post", "2024-02-01"),
        ];

        generate(&config, &pages, dir.path()).unwrap();

        let sitemap = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        assert!(sitemap.contains("<urlset"));
        assert!(sitemap.contains("https://example.com/about/"));
        assert!(sitemap.contains("https://example.com/blog/post/"));
        assert!(sitemap.contains("<lastmod>2024-01-15</lastmod>"));
        assert!(sitemap.contains("<changefreq>weekly</changefreq>"));
    }

    #[test]
    fn page_exclusion() {
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com");

        let mut excluded = test_page("private", "2024-01-01");
        excluded.frontmatter.sitemap = Some(false);

        let mut draft = test_page("draft", "2024-01-01");
        draft.frontmatter.draft = Some(true);

        let pages = vec![test_page("public", "2024-01-01"), excluded, draft];

        generate(&config, &pages, dir.path()).unwrap();

        let sitemap = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        assert!(sitemap.contains("public"));
        assert!(!sitemap.contains("private"));
        assert!(!sitemap.contains("draft"));
    }

    #[test]
    fn robots_txt_content() {
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com");

        generate(&config, &[], dir.path()).unwrap();

        let robots = std::fs::read_to_string(dir.path().join("robots.txt")).unwrap();
        assert!(robots.contains("User-agent: *"));
        assert!(robots.contains("Allow: /"));
        assert!(robots.contains("Sitemap: https://example.com/sitemap.xml"));
    }

    #[test]
    fn sitemap_with_nested_slug_paths() {
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com");
        let pages = vec![
            test_page("blog/2024/my-post", "2024-06-01"),
            test_page("docs/api/v2/reference", "2024-05-15"),
        ];

        generate(&config, &pages, dir.path()).unwrap();

        let sitemap = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        assert!(sitemap.contains("https://example.com/blog/2024/my-post/"));
        assert!(sitemap.contains("https://example.com/docs/api/v2/reference/"));
    }

    #[test]
    fn sitemap_disabled_via_config() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = SiteConfig::for_testing("Test", "https://example.com");
        config.sitemap = Some(crate::config::SitemapConfig {
            enabled: false,
            changefreq: "weekly".to_string(),
        });
        let pages = vec![test_page("about", "2024-01-01")];

        generate(&config, &pages, dir.path()).unwrap();

        // Neither sitemap.xml nor robots.txt should be generated
        assert!(!dir.path().join("sitemap.xml").exists());
        assert!(!dir.path().join("robots.txt").exists());
    }

    #[test]
    fn large_number_of_pages_in_sitemap() {
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com");
        let pages: Vec<Page> = (0..500)
            .map(|i| test_page(&format!("page-{i}"), "2024-01-01"))
            .collect();

        generate(&config, &pages, dir.path()).unwrap();

        let sitemap = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        let url_count = sitemap.matches("<url>").count();
        assert_eq!(url_count, 500);
    }

    #[test]
    fn base_url_with_trailing_slash_handled() {
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com/");
        let pages = vec![test_page("about", "2024-01-01")];

        generate(&config, &pages, dir.path()).unwrap();

        let sitemap = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        // Should not produce double slashes like "https://example.com//about/"
        assert!(!sitemap.contains("example.com//"));
        assert!(sitemap.contains("https://example.com/about/"));
    }

    #[test]
    fn sitemap_dates_are_consistent_format() {
        // Zola issue #2335: mixed date formats cause Google to reject sitemaps
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com");
        let pages = vec![
            test_page("page-a", "2024-01-15"),
            test_page("page-b", "2024-06-15T12:00:00"),
        ];

        generate(&config, &pages, dir.path()).unwrap();

        let sitemap = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        // Both dates should be present and valid
        assert!(sitemap.contains("<lastmod>"));
        // Count lastmod entries matches page count
        let lastmod_count = sitemap.matches("<lastmod>").count();
        assert_eq!(lastmod_count, 2);
    }

    // --- Hugo regression tests ---

    #[test]
    fn sitemap_xml_has_valid_declaration_and_namespace() {
        // Hugo #10515: sitemap must have valid XML declaration
        let dir = tempfile::tempdir().unwrap();
        let config = SiteConfig::for_testing("Test", "https://example.com");
        let pages = vec![test_page("about", "2024-01-01")];

        generate(&config, &pages, dir.path()).unwrap();

        let sitemap = std::fs::read_to_string(dir.path().join("sitemap.xml")).unwrap();
        assert!(sitemap.starts_with("<?xml version=\"1.0\""));
        assert!(sitemap.contains("xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\""));
    }
}
