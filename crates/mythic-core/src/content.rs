//! Content discovery — walks the content directory and builds Page structs.
//!
//! Uses lasso string interning to deduplicate repeated frontmatter values
//! (layout names, tag strings) across pages, reducing heap allocations.

use anyhow::{Context, Result};
use lasso::ThreadedRodeo;
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

    // String interner for deduplicating repeated frontmatter values.
    let interner = ThreadedRodeo::default();

    // Collect file paths in a single walkdir pass
    let paths: Vec<std::path::PathBuf> = WalkDir::new(&content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            !name.starts_with('.') && !name.starts_with('_')
        })
        .filter(|e| {
            matches!(
                e.path().extension().and_then(|x| x.to_str()),
                Some("md" | "markdown")
            )
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let mut pages = Vec::with_capacity(paths.len());

    // Reusable hasher state — avoids re-creating RandomState per file
    let hash_state = ahash::RandomState::with_seeds(
        0x517cc1b727220a95,
        0x6c62272e07bb0142,
        0x2f8e4c5d6a3b1e09,
        0x9d8c7b6a5f4e3d2c,
    );

    for path in &paths {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;
        let raw = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw).to_string();

        let content_hash = hash_state.hash_one(&raw);

        let rel = path.strip_prefix(&content_dir).unwrap_or(path);
        let slug = rel.with_extension("").to_string_lossy().replace('\\', "/");

        let (mut frontmatter, body) = mythic_markdown_parse_stub(&raw);

        // Intern repeated strings to deduplicate heap allocations
        intern_frontmatter(&interner, &mut frontmatter);

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

/// Intern frequently repeated frontmatter strings (layout, tags).
/// Resolves each string through the interner so identical values
/// across pages share the same heap allocation.
fn intern_frontmatter(interner: &ThreadedRodeo, fm: &mut crate::page::Frontmatter) {
    use compact_str::CompactString;

    // Intern layout name (most pages use "default")
    if let Some(ref layout) = fm.layout {
        let key = interner.get_or_intern(layout.as_str());
        fm.layout = Some(CompactString::from(interner.resolve(&key)));
    }

    // Intern tag values (many pages share common tags)
    if let Some(ref mut tags) = fm.tags {
        for tag in tags.iter_mut() {
            let key = interner.get_or_intern(tag.as_str());
            *tag = CompactString::from(interner.resolve(&key));
        }
    }
}

/// Lightweight frontmatter extraction for the discovery phase.
fn mythic_markdown_parse_stub(raw: &str) -> (crate::page::Frontmatter, String) {
    use crate::page::Frontmatter;

    if let Some(after_open) = raw.strip_prefix("---") {
        if let Some(end) = after_open.find("\n---") {
            let yaml_str = &after_open[..end];
            let body = after_open[end + 4..].trim_start().to_string();
            if let Ok(fm) = serde_yaml::from_str::<Frontmatter>(yaml_str) {
                return (fm, body);
            }
        }
    }

    if let Some(after_open) = raw.strip_prefix("+++") {
        if let Some(end) = after_open.find("\n+++") {
            let toml_str = &after_open[..end];
            let body = after_open[end + 4..].trim_start().to_string();
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

    #[test]
    fn slug_preserves_underscores() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("rust-cfg_attr.md"), "# Test").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages[0].slug, "rust-cfg_attr");
    }

    #[test]
    fn empty_markdown_file_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("empty.md"), "").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn file_with_bom_marker_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        let mut bom_content = vec![0xEF, 0xBB, 0xBF];
        bom_content.extend_from_slice(b"---\ntitle: BOM Post\n---\nBody after BOM");
        std::fs::write(content.join("bom.md"), bom_content).unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].frontmatter.title, "BOM Post");
        assert_eq!(pages[0].raw_content, "Body after BOM");
    }

    #[test]
    fn content_with_frontmatter_only_produces_empty_body() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("meta.md"), "---\ntitle: Meta Only\n---\n").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].raw_content.trim().is_empty());
    }

    #[test]
    fn editor_temp_files_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("post.md"), "# Real").unwrap();
        std::fs::write(content.join("post.md~"), "# Backup").unwrap();
        std::fs::write(content.join(".post.md.swp"), "# Vim swap").unwrap();
        std::fs::write(content.join("#post.md#"), "# Emacs auto-save").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "post");
    }

    #[test]
    fn slug_strips_leading_trailing_hyphens() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("--hello-world--.md"), "# Test").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages[0].slug, "--hello-world--");
    }

    #[test]
    fn urls_use_forward_slashes() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("content/blog/post");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("index.md"), "# Test").unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert!(
            !pages[0].slug.contains('\\'),
            "Slug should use / not \\: {}",
            pages[0].slug
        );
    }

    #[test]
    fn string_interning_deduplicates_layouts() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();

        // Create multiple pages with the same layout
        for i in 0..10 {
            std::fs::write(
                content.join(format!("page-{i}.md")),
                "---\ntitle: Test\nlayout: blog\ntags:\n  - rust\n  - web\n---\nBody",
            )
            .unwrap();
        }

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 10);

        // All pages should have layout "blog" and tags ["rust", "web"]
        for page in &pages {
            assert_eq!(page.frontmatter.layout.as_deref(), Some("blog"));
            assert_eq!(page.frontmatter.tags.as_ref().unwrap(), &["rust", "web"]);
        }
    }

    #[test]
    fn frontmatter_with_triple_dash_in_yaml_content() {
        // The `\n---` fix: frontmatter delimiter must start at beginning of line
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        // YAML value contains "---" but not at the start of a line
        std::fs::write(
            content.join("tricky.md"),
            "---\ntitle: \"A title with --- dashes\"\ndescription: \"Has --- in it\"\n---\nBody text here",
        )
        .unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].frontmatter.title, "A title with --- dashes");
        assert_eq!(pages[0].raw_content, "Body text here");
    }

    #[test]
    fn bom_stripped_then_frontmatter_parses_correctly() {
        // Verify the full chain: BOM is stripped AND frontmatter still parses
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();

        // UTF-8 BOM followed by YAML frontmatter with tags
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(
            b"---\ntitle: BOM Test\ntags:\n  - rust\n  - web\n---\nContent after BOM",
        );
        std::fs::write(content.join("bom-tags.md"), data).unwrap();

        let config = fixture_config();
        let pages = discover_content(&config, dir.path()).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].frontmatter.title, "BOM Test");
        assert_eq!(
            pages[0].frontmatter.tags.as_ref().unwrap(),
            &["rust", "web"]
        );
        assert_eq!(pages[0].raw_content, "Content after BOM");
    }
}
