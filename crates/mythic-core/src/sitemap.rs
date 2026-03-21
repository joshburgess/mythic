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

        let url = format!("{base_url}/{}/", page.slug);
        let lastmod = page
            .frontmatter
            .date
            .as_deref()
            .unwrap_or("2024-01-01");

        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{url}</loc>\n"));
        xml.push_str(&format!("    <lastmod>{lastmod}</lastmod>\n"));
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
    let content = format!(
        "User-agent: *\nAllow: /\n\nSitemap: {base_url}/sitemap.xml\n"
    );

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
                title: slug.to_string(),
                date: Some(date.to_string()),
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

        let pages = vec![
            test_page("public", "2024-01-01"),
            excluded,
            draft,
        ];

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
}
