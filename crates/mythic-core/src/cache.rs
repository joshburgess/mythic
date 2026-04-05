//! Incremental build cache using content hashes.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

const CACHE_FILENAME: &str = ".mythic-cache.json";

/// Dependency graph tracking content hashes for incremental builds.
#[derive(Debug, Serialize, Deserialize)]
pub struct DepGraph {
    hashes: HashMap<String, u64>,
    /// Hash of the config file — if this changes, all pages are invalidated.
    #[serde(default)]
    config_hash: u64,
    /// Hash of all non-content files (templates, config, styles, scripts, shortcodes).
    /// When this changes, the entire page cache is invalidated.
    #[serde(default)]
    pub env_hash: u64,
    #[serde(skip)]
    path: PathBuf,
    /// When true, all pages are treated as changed (config/template change).
    #[serde(skip)]
    force_rebuild: bool,
}

impl DepGraph {
    /// Load the cache from the output directory, or create an empty one.
    pub fn load(output_dir: &Path) -> Self {
        let path = output_dir.join(CACHE_FILENAME);
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(mut graph) = serde_json::from_str::<DepGraph>(&data) {
                    graph.path = path;
                    return graph;
                }
            }
        }
        DepGraph {
            hashes: HashMap::new(),
            config_hash: 0,
            env_hash: 0,
            path,
            force_rebuild: false,
        }
    }

    /// Set the current config hash. If it differs from the cached value,
    /// all pages will be treated as changed.
    pub fn set_config_hash(&mut self, hash: u64) {
        if self.config_hash != hash {
            self.force_rebuild = true;
            self.config_hash = hash;
        }
    }

    /// Check if a page's content has changed since the last build.
    pub fn is_changed(&self, slug: &str, content_hash: u64) -> bool {
        if self.force_rebuild {
            return true;
        }
        match self.hashes.get(slug) {
            Some(&cached_hash) => cached_hash != content_hash,
            None => true,
        }
    }

    /// Check the environment hash and invalidate all page hashes if it changed.
    /// Should be called once after loading, before any `is_changed` calls.
    pub fn check_env(&mut self, current_env_hash: u64) {
        if self.env_hash != current_env_hash {
            self.hashes.clear();
            self.env_hash = current_env_hash;
        }
    }

    /// Record a page's hash after writing it.
    pub fn record(&mut self, slug: &str, content_hash: u64) {
        self.hashes.insert(slug.to_string(), content_hash);
    }

    /// Remove orphaned entries from the cache whose slugs no longer exist in
    /// the current set of pages, and delete the corresponding HTML files from
    /// the output directory.
    ///
    /// Note: this only operates on content page hashes recorded via `record()`.
    /// Taxonomy pages (tags, categories, etc.) are generated in post-build and
    /// are not tracked in the cache, so they are unaffected by orphan cleanup.
    pub fn remove_orphans(&mut self, current_slugs: &[&str], output_dir: &Path, ugly_urls: bool) {
        let current: std::collections::HashSet<&str> = current_slugs.iter().copied().collect();
        let orphaned: Vec<String> = self
            .hashes
            .keys()
            .filter(|slug| !current.contains(slug.as_str()))
            .cloned()
            .collect();

        for slug in &orphaned {
            // Delete the stale HTML file
            let path = if ugly_urls {
                output_dir.join(format!("{slug}.html"))
            } else if slug == "index" {
                output_dir.join("index.html")
            } else {
                output_dir.join(slug).join("index.html")
            };
            let _ = std::fs::remove_file(&path);

            // Also try to remove the now-empty parent directory (clean URLs)
            if !ugly_urls && slug != "index" {
                let dir = output_dir.join(slug);
                let _ = std::fs::remove_dir(&dir);
            }

            self.hashes.remove(slug);
        }
    }

    /// Save the cache to disk.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create cache directory: {}", parent.display())
            })?;
        }
        let json =
            serde_json::to_string_pretty(&self).context("Failed to serialize build cache")?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("Failed to write cache: {}", self.path.display()))?;
        Ok(())
    }
}

/// Compute an environment hash from all non-content files that affect the build
/// output: templates, config, styles, scripts, and shortcodes.
pub fn compute_env_hash(root: &Path, config: &crate::config::SiteConfig) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    let dirs_to_hash = [
        root.join(&config.template_dir),
        root.join(&config.styles_dir),
        root.join(&config.scripts_dir),
        root.join(&config.data_dir),
        root.join("shortcodes"),
    ];

    // Hash the config file itself
    let config_path = root.join("mythic.toml");
    if let Ok(content) = std::fs::read(&config_path) {
        config_path.to_string_lossy().hash(&mut hasher);
        content.hash(&mut hasher);
    }

    // Hash the effective base_url and base_path so that switching between
    // mythic build (production URL) and mythic serve (localhost) invalidates
    // the cache, since these affect asset paths and link URLs in output.
    config.base_url.hash(&mut hasher);
    config.base_path().hash(&mut hasher);

    // Hash all files in template, style, script, and shortcode directories
    for dir in &dirs_to_hash {
        if !dir.exists() {
            continue;
        }
        if let Ok(walker) = walkdir_sorted(dir) {
            for entry in walker {
                if let Ok(content) = std::fs::read(entry.path()) {
                    entry.path().to_string_lossy().hash(&mut hasher);
                    content.hash(&mut hasher);
                }
            }
        }
    }

    hasher.finish()
}

/// Walk a directory and return file entries sorted by path for deterministic hashing.
fn walkdir_sorted(dir: &Path) -> Result<Vec<walkdir::DirEntry>> {
    let mut entries: Vec<walkdir::DirEntry> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cache_reports_all_changed() {
        let dir = tempfile::tempdir().unwrap();
        let graph = DepGraph::load(dir.path());
        assert!(graph.is_changed("hello", 12345));
    }

    #[test]
    fn recorded_hash_reports_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let mut graph = DepGraph::load(dir.path());
        graph.record("hello", 12345);
        assert!(!graph.is_changed("hello", 12345));
    }

    #[test]
    fn different_hash_reports_changed() {
        let dir = tempfile::tempdir().unwrap();
        let mut graph = DepGraph::load(dir.path());
        graph.record("hello", 12345);
        assert!(graph.is_changed("hello", 99999));
    }

    #[test]
    fn round_trip_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let mut graph = DepGraph::load(dir.path());
        graph.record("page-a", 111);
        graph.record("page-b", 222);
        graph.save().unwrap();

        let reloaded = DepGraph::load(dir.path());
        assert!(!reloaded.is_changed("page-a", 111));
        assert!(!reloaded.is_changed("page-b", 222));
        assert!(reloaded.is_changed("page-a", 999));
    }

    #[test]
    fn corrupted_cache_file_handled_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join(CACHE_FILENAME);

        // Write invalid JSON
        std::fs::write(&cache_path, "this is not valid json {{{").unwrap();

        // Should not panic, should return empty graph
        let graph = DepGraph::load(dir.path());
        assert!(graph.is_changed("anything", 0));
        assert!(graph.hashes.is_empty());
    }

    #[test]
    fn cache_with_many_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut graph = DepGraph::load(dir.path());

        for i in 0..1000 {
            graph.record(&format!("page-{i}"), i as u64);
        }
        graph.save().unwrap();

        let reloaded = DepGraph::load(dir.path());
        for i in 0..1000 {
            assert!(
                !reloaded.is_changed(&format!("page-{i}"), i as u64),
                "page-{i} should be unchanged"
            );
        }
        // Verify a changed one is detected
        assert!(reloaded.is_changed("page-500", 99999));
    }

    #[test]
    fn cache_is_a_plain_hashmap() {
        // Verify that the cache is just a HashMap and behaves as expected
        // with overwrites
        let dir = tempfile::tempdir().unwrap();
        let mut graph = DepGraph::load(dir.path());

        graph.record("page", 100);
        assert!(!graph.is_changed("page", 100));

        // Overwrite with new hash
        graph.record("page", 200);
        assert!(graph.is_changed("page", 100));
        assert!(!graph.is_changed("page", 200));

        // Save and reload to verify persistence of overwrite
        graph.save().unwrap();
        let reloaded = DepGraph::load(dir.path());
        assert!(reloaded.is_changed("page", 100));
        assert!(!reloaded.is_changed("page", 200));
    }

    #[test]
    fn env_hash_invalidates_all_pages() {
        let dir = tempfile::tempdir().unwrap();
        let mut graph = DepGraph::load(dir.path());
        graph.env_hash = 100;
        graph.record("page-a", 111);
        graph.record("page-b", 222);
        graph.save().unwrap();

        let mut reloaded = DepGraph::load(dir.path());
        // Same env hash — pages should be cached
        reloaded.check_env(100);
        assert!(!reloaded.is_changed("page-a", 111));
        assert!(!reloaded.is_changed("page-b", 222));

        // Different env hash — all pages should be invalidated
        let mut reloaded2 = DepGraph::load(dir.path());
        reloaded2.check_env(999);
        assert!(reloaded2.is_changed("page-a", 111));
        assert!(reloaded2.is_changed("page-b", 222));
    }

    #[test]
    fn env_hash_changes_when_template_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create minimal config
        std::fs::write(
            root.join("mythic.toml"),
            "title = \"T\"\nbase_url = \"http://x.com\"\n",
        )
        .unwrap();

        // Create a template directory with a file
        let tmpl_dir = root.join("templates");
        std::fs::create_dir_all(&tmpl_dir).unwrap();
        std::fs::write(tmpl_dir.join("base.html"), "<html>v1</html>").unwrap();

        let config = crate::config::load_config(&root.join("mythic.toml")).unwrap();
        let hash1 = crate::cache::compute_env_hash(root, &config);

        // Change the template file content
        std::fs::write(tmpl_dir.join("base.html"), "<html>v2</html>").unwrap();
        let hash2 = crate::cache::compute_env_hash(root, &config);

        assert_ne!(
            hash1, hash2,
            "env_hash should change when a template file changes"
        );
    }

    #[test]
    fn orphan_cleanup_removes_stale_html() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path();

        let mut graph = DepGraph::load(output);
        graph.record("about", 111);
        graph.record("old-post", 222);

        // Create the corresponding output files (clean URLs)
        let about_dir = output.join("about");
        std::fs::create_dir_all(&about_dir).unwrap();
        std::fs::write(about_dir.join("index.html"), "<p>about</p>").unwrap();

        let old_dir = output.join("old-post");
        std::fs::create_dir_all(&old_dir).unwrap();
        std::fs::write(old_dir.join("index.html"), "<p>old</p>").unwrap();

        // Also create a non-content file that should be preserved
        std::fs::write(output.join("style.css"), "body {}").unwrap();

        // Remove orphans: only "about" still exists
        graph.remove_orphans(&["about"], output, false);

        // "about" should still be there
        assert!(about_dir.join("index.html").exists());
        // "old-post" should be removed
        assert!(!old_dir.join("index.html").exists());
        // Non-content file preserved
        assert!(output.join("style.css").exists());
        // Cache should no longer contain "old-post"
        assert!(graph.is_changed("old-post", 222));
        assert!(!graph.is_changed("about", 111));
    }

    #[test]
    fn orphan_cleanup_ugly_urls_mode() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path();

        let mut graph = DepGraph::load(output);
        graph.record("about", 111);
        graph.record("old-post", 222);

        // In ugly_urls mode, output is slug.html directly
        std::fs::write(output.join("about.html"), "<p>about</p>").unwrap();
        std::fs::write(output.join("old-post.html"), "<p>old</p>").unwrap();

        graph.remove_orphans(&["about"], output, true);

        assert!(output.join("about.html").exists());
        assert!(!output.join("old-post.html").exists());
    }
}
