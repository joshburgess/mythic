//! Incremental build cache using content hashes.
//!
//! Tracks per-page content hashes for incremental builds and an environment
//! hash covering templates, config, styles, scripts, and shortcodes. When any
//! non-content file changes, the entire cache is invalidated.

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
    /// Hash of all non-content files (templates, config, styles, scripts, shortcodes).
    /// When this changes, the entire page cache is invalidated.
    #[serde(default)]
    env_hash: u64,
    #[serde(skip)]
    path: PathBuf,
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
            env_hash: 0,
            path,
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

    /// Check if a page's content has changed since the last build.
    pub fn is_changed(&self, slug: &str, content_hash: u64) -> bool {
        match self.hashes.get(slug) {
            Some(&cached_hash) => cached_hash != content_hash,
            None => true,
        }
    }

    /// Record a page's hash after writing it.
    pub fn record(&mut self, slug: &str, content_hash: u64) {
        self.hashes.insert(slug.to_string(), content_hash);
    }

    /// Remove orphaned entries from the cache whose slugs no longer exist in
    /// the current set of pages, and delete the corresponding HTML files from
    /// the output directory.
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
        root.join("shortcodes"),
    ];

    // Hash the config file itself
    let config_path = root.join("mythic.toml");
    if let Ok(content) = std::fs::read(&config_path) {
        config_path.to_string_lossy().hash(&mut hasher);
        content.hash(&mut hasher);
    }

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
}
