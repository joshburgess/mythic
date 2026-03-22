//! Incremental build cache using content hashes.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CACHE_FILENAME: &str = ".mythic-cache.json";

/// Dependency graph tracking content hashes for incremental builds.
#[derive(Debug, Serialize, Deserialize)]
pub struct DepGraph {
    hashes: HashMap<String, u64>,
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
            path,
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
}
