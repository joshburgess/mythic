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
    let mut pages = Vec::new();

    if !content_dir.exists() {
        return Ok(pages);
    }

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

        // Derive slug from relative path
        let rel = path
            .strip_prefix(&content_dir)
            .unwrap_or(path);
        let slug = rel
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        // Parse frontmatter — we'll do full parsing via mythic-markdown,
        // but for discovery we do a lightweight split.
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
/// Full parsing with validation lives in mythic-markdown.
fn mythic_markdown_parse_stub(raw: &str) -> (crate::page::Frontmatter, String) {
    use crate::page::Frontmatter;

    // Try YAML frontmatter (---)
    if raw.starts_with("---") {
        if let Some(end) = raw[3..].find("---") {
            let yaml_str = &raw[3..3 + end];
            let body = raw[3 + end + 3..].trim_start().to_string();
            if let Ok(fm) = serde_yaml::from_str::<Frontmatter>(yaml_str) {
                return (fm, body);
            }
        }
    }

    // Try TOML frontmatter (+++)
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
    use std::path::PathBuf;

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
}
