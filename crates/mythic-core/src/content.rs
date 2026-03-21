//! Content discovery — walks the content directory and builds Page structs.

use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use walkdir::WalkDir;

use crate::config::SiteConfig;
use crate::page::Page;

/// Discover all markdown content files and return parsed Pages.
pub fn discover_content(config: &SiteConfig, root: &Path) -> Result<Vec<Page>> {
    let content_dir = root.join(&config.content_dir);

    if !content_dir.exists() {
        return Ok(Vec::new());
    }

    let mut pages = Vec::new();

    for entry in WalkDir::new(&content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip hidden files and files starting with _
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name.starts_with('_') {
                continue;
            }
        }

        // Only process markdown files
        match path.extension().and_then(|e| e.to_str()) {
            Some("md" | "markdown") => {}
            _ => continue,
        }

        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;

        let content_hash = {
            let mut hasher = DefaultHasher::new();
            raw.hash(&mut hasher);
            hasher.finish()
        };

        let rel = path.strip_prefix(&content_dir).unwrap_or(path);
        let slug = rel
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        let (frontmatter, body) = mythic_markdown_parse_stub(&raw);

        pages.push(Page {
            source_path: path.to_path_buf(),
            slug,
            frontmatter,
            raw_content: body,
            rendered_html: None,
            output_path: None,
            content_hash,
            toc: Vec::new(),
        });
    }

    Ok(pages)
}

/// Lightweight frontmatter extraction for the discovery phase.
fn mythic_markdown_parse_stub(raw: &str) -> (crate::page::Frontmatter, String) {
    use crate::page::Frontmatter;

    if raw.starts_with("---") {
        if let Some(end) = raw[3..].find("---") {
            let yaml_str = &raw[3..3 + end];
            let body = raw[3 + end + 3..].trim_start().to_string();
            if let Ok(fm) = serde_yaml::from_str::<Frontmatter>(yaml_str) {
                return (fm, body);
            }
        }
    }

    if raw.starts_with("+++") {
        if let Some(end) = raw[3..].find("+++") {
            let toml_str = &raw[3..3 + end];
            let body = raw[3 + end + 3..].trim_start().to_string();
            if let Ok(fm) = toml::from_str::<Frontmatter>(toml_str) {
                return (fm, body);
            }
        }
    }

    (Frontmatter::default(), raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SiteConfig;

    fn fixture_config() -> SiteConfig {
        SiteConfig::for_testing("Test", "http://localhost")
    }

    #[test]
    fn discovers_fixture_content() {
        let config = fixture_config();
        let root = Path::new("../../fixtures/basic-site");
        let pages = discover_content(&config, root).unwrap();
        assert!(!pages.is_empty());
        assert!(pages.iter().any(|p| p.slug == "hello"));
    }

    #[test]
    fn skips_hidden_and_underscore_files() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join(".hidden.md"), "# Hidden").unwrap();
        std::fs::write(content.join("_draft.md"), "# Draft").unwrap();
        std::fs::write(content.join("visible.md"), "# Visible").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "visible");
    }

    #[test]
    fn handles_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("content/blog/2024");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("post.md"), "# Post").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "blog/2024/post");
    }

    #[test]
    fn non_markdown_files_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("page.txt"), "text file").unwrap();
        std::fs::write(content.join("page.html"), "<p>html</p>").unwrap();
        std::fs::write(content.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(content.join("real.md"), "# Real").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "real");
    }

    #[test]
    fn markdown_extension_discovered() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("page.markdown"), "# Markdown ext").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "page");
    }

    #[test]
    fn toml_frontmatter_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("post.md"),
            "+++\ntitle = \"TOML Post\"\ndraft = true\n+++\nBody here",
        )
        .unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].frontmatter.title, "TOML Post");
        assert_eq!(pages[0].frontmatter.draft, Some(true));
        assert_eq!(pages[0].raw_content, "Body here");
    }

    #[test]
    fn unicode_filenames_work() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("über-uns.md"), "# Über Uns").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].slug.contains("über-uns"));
    }

    #[test]
    fn deeply_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("content/a/b/c/d/e");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("leaf.md"), "# Leaf").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "a/b/c/d/e/leaf");
    }

    #[test]
    fn large_number_of_files_in_flat_directory() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();

        for i in 0..100 {
            std::fs::write(content.join(format!("page-{i}.md")), format!("# Page {i}")).unwrap();
        }

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 100);
    }

    #[test]
    fn content_hash_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("page.md"), "# Deterministic").unwrap();

        let config = fixture_config();
        let pages1 = discover_content(&config, dir.path()).unwrap();
        let pages2 = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages1[0].content_hash, pages2[0].content_hash);
    }

    #[test]
    fn content_hash_changes_when_content_changes() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("page.md"), "# Version 1").unwrap();

        let config = fixture_config();
        let pages1 = discover_content(&config, dir.path()).unwrap();
        let hash1 = pages1[0].content_hash;

        std::fs::write(content.join("page.md"), "# Version 2").unwrap();
        let pages2 = discover_content(&config, dir.path()).unwrap();
        let hash2 = pages2[0].content_hash;

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn slug_derivation_from_various_paths() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(content.join("blog/2024")).unwrap();
        std::fs::create_dir_all(content.join("docs")).unwrap();

        std::fs::write(content.join("index.md"), "# Home").unwrap();
        std::fs::write(content.join("blog/2024/hello-world.md"), "# HW").unwrap();
        std::fs::write(content.join("docs/getting-started.md"), "# GS").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        let slugs: Vec<&str> = pages.iter().map(|p| p.slug.as_str()).collect();

        assert!(slugs.contains(&"index"));
        assert!(slugs.contains(&"blog/2024/hello-world"));
        assert!(slugs.contains(&"docs/getting-started"));
    }
}
