//! Redirect/alias generation from frontmatter `aliases` field.

use anyhow::Result;
use std::path::Path;

use crate::page::Page;

/// Generate HTML redirect files for all pages with aliases.
///
/// For each alias URL, generates an HTML file with a `<meta http-equiv="refresh">`
/// redirect to the page's canonical URL.
pub fn generate_redirects(pages: &[Page], output_dir: &Path, base_url: &str) -> Result<usize> {
    let base_url = base_url.trim_end_matches('/');
    let mut count = 0;

    for page in pages {
        let aliases = match &page.frontmatter.aliases {
            Some(a) if !a.is_empty() => a,
            _ => continue,
        };

        let canonical_url = format!("{base_url}/{}/", page.slug);

        for alias in aliases {
            let alias_path = alias.trim_matches('/');
            let dest = output_dir.join(alias_path).join("index.html");

            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let html = format!(
                r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta http-equiv="refresh" content="0; url={canonical_url}">
<link rel="canonical" href="{canonical_url}">
<title>Redirect</title>
</head>
<body>
<p>This page has moved to <a href="{canonical_url}">{canonical_url}</a>.</p>
</body>
</html>
"#
            );

            std::fs::write(&dest, html)?;
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn page_with_aliases(slug: &str, aliases: Vec<&str>) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: "Test".into(),
                aliases: Some(aliases.into_iter().map(String::from).collect()),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: None,
            body_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn generates_redirect_html() {
        let dir = tempfile::tempdir().unwrap();
        let pages = vec![page_with_aliases("blog/new-post", vec!["/old-post/"])];

        let count = generate_redirects(&pages, dir.path(), "https://example.com").unwrap();
        assert_eq!(count, 1);

        let redirect = std::fs::read_to_string(dir.path().join("old-post/index.html")).unwrap();
        assert!(redirect.contains("http-equiv=\"refresh\""));
        assert!(redirect.contains("https://example.com/blog/new-post/"));
        assert!(redirect.contains("rel=\"canonical\""));
    }

    #[test]
    fn multiple_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let pages = vec![page_with_aliases(
            "docs/guide",
            vec!["/getting-started/", "/tutorial/", "/docs/intro/"],
        )];

        let count = generate_redirects(&pages, dir.path(), "https://example.com").unwrap();
        assert_eq!(count, 3);
        assert!(dir.path().join("getting-started/index.html").exists());
        assert!(dir.path().join("tutorial/index.html").exists());
        assert!(dir.path().join("docs/intro/index.html").exists());
    }

    #[test]
    fn no_aliases_produces_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let pages = vec![Page {
            source_path: PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: Frontmatter {
                title: "Test".into(),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: None,
            body_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }];

        let count = generate_redirects(&pages, dir.path(), "https://example.com").unwrap();
        assert_eq!(count, 0);
    }
}
