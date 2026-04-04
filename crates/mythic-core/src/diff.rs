//! Smart content diffing for minimal deployments.
//!
//! Generates a manifest of exactly which output files changed between
//! builds, enabling rsync-style minimal deployments where only modified
//! files are uploaded.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DIFF_MANIFEST_FILE: &str = ".mythic-diff.json";

/// A record of all output files and their content hashes.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DiffManifest {
    pub files: HashMap<String, String>,
}

/// The result of comparing two manifests.
#[derive(Debug, Default, Serialize)]
pub struct DiffResult {
    /// Files that were added (not in previous build).
    pub added: Vec<String>,
    /// Files that were modified (hash changed).
    pub modified: Vec<String>,
    /// Files that were removed (in previous but not current).
    pub removed: Vec<String>,
    /// Files that are unchanged.
    pub unchanged: usize,
}

impl DiffResult {
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.modified.len() + self.removed.len()
    }
}

impl std::fmt::Display for DiffResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n  Content diff:")?;
        writeln!(f, "    Added:     {}", self.added.len())?;
        writeln!(f, "    Modified:  {}", self.modified.len())?;
        writeln!(f, "    Removed:   {}", self.removed.len())?;
        writeln!(f, "    Unchanged: {}", self.unchanged)
    }
}

/// Scan the output directory, compute hashes for all files, and compare
/// against the previous manifest to determine what changed.
///
/// Saves the new manifest for the next comparison.
pub fn compute_diff(output_dir: &Path) -> Result<DiffResult> {
    let manifest_path = output_dir.join(DIFF_MANIFEST_FILE);

    // Load previous manifest
    let previous = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)?;
        serde_json::from_str::<DiffManifest>(&content).unwrap_or_default()
    } else {
        DiffManifest::default()
    };

    // Build current manifest
    let current = build_manifest(output_dir)?;

    // Compare
    let result = compare_manifests(&previous, &current);

    // Save current manifest for next build
    let json = serde_json::to_string_pretty(&current)?;
    std::fs::write(&manifest_path, json)?;

    Ok(result)
}

/// Generate a deployable file list (only changed files).
///
/// Writes `deploy-manifest.json` to the output directory with the list
/// of files that need to be uploaded/deleted.
pub fn write_deploy_manifest(output_dir: &Path, diff: &DiffResult) -> Result<PathBuf> {
    let manifest = serde_json::json!({
        "upload": diff.added.iter().chain(diff.modified.iter()).collect::<Vec<_>>(),
        "delete": &diff.removed,
        "total_upload": diff.added.len() + diff.modified.len(),
        "total_delete": diff.removed.len(),
    });

    let path = output_dir.join("deploy-manifest.json");
    std::fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(path)
}

fn build_manifest(output_dir: &Path) -> Result<DiffManifest> {
    use std::hash::{BuildHasher, Hasher};

    let hash_state = ahash::RandomState::with_seeds(
        0x12345678_9abcdef0,
        0xfedcba98_76543210,
        0x0a1b2c3d_4e5f6a7b,
        0x8c9daebf_0c1d2e3f,
    );

    let mut files = HashMap::new();

    for entry in WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(output_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Skip the manifest files themselves
        if rel == DIFF_MANIFEST_FILE || rel == "deploy-manifest.json" {
            continue;
        }

        if let Ok(content) = std::fs::read(path) {
            let mut hasher = hash_state.build_hasher();
            std::hash::Hash::hash_slice(&content, &mut hasher);
            let hash = format!("{:x}", hasher.finish());
            files.insert(rel, hash);
        }
    }

    Ok(DiffManifest { files })
}

fn compare_manifests(previous: &DiffManifest, current: &DiffManifest) -> DiffResult {
    let mut result = DiffResult::default();

    // Check for added and modified files
    for (path, hash) in &current.files {
        match previous.files.get(path) {
            None => result.added.push(path.clone()),
            Some(prev_hash) if prev_hash != hash => result.modified.push(path.clone()),
            _ => result.unchanged += 1,
        }
    }

    // Check for removed files
    for path in previous.files.keys() {
        if !current.files.contains_key(path) {
            result.removed.push(path.clone());
        }
    }

    result.added.sort();
    result.modified.sort();
    result.removed.sort();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_build_all_added() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>Hello</h1>").unwrap();
        std::fs::write(dir.path().join("about.html"), "<h1>About</h1>").unwrap();

        let diff = compute_diff(dir.path()).unwrap();
        assert_eq!(diff.added.len(), 2);
        assert_eq!(diff.modified.len(), 0);
        assert_eq!(diff.removed.len(), 0);
    }

    #[test]
    fn no_changes_all_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>Hello</h1>").unwrap();

        compute_diff(dir.path()).unwrap(); // first build
        let diff = compute_diff(dir.path()).unwrap(); // second build

        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.modified.len(), 0);
        assert_eq!(diff.unchanged, 1);
    }

    #[test]
    fn modified_file_detected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>V1</h1>").unwrap();

        compute_diff(dir.path()).unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>V2</h1>").unwrap();

        let diff = compute_diff(dir.path()).unwrap();
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0], "index.html");
    }

    #[test]
    fn removed_file_detected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "A").unwrap();
        std::fs::write(dir.path().join("old.html"), "B").unwrap();

        compute_diff(dir.path()).unwrap();
        std::fs::remove_file(dir.path().join("old.html")).unwrap();

        let diff = compute_diff(dir.path()).unwrap();
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0], "old.html");
    }

    #[test]
    fn deploy_manifest_generated() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("new.html"), "new").unwrap();

        let diff = compute_diff(dir.path()).unwrap();
        let manifest_path = write_deploy_manifest(dir.path(), &diff).unwrap();
        assert!(manifest_path.exists());

        let content = std::fs::read_to_string(manifest_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["total_upload"].as_u64().unwrap() > 0);
    }

    #[test]
    fn mixed_changes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("keep.html"), "same").unwrap();
        std::fs::write(dir.path().join("change.html"), "v1").unwrap();
        std::fs::write(dir.path().join("delete.html"), "gone").unwrap();

        compute_diff(dir.path()).unwrap();

        std::fs::write(dir.path().join("change.html"), "v2").unwrap();
        std::fs::remove_file(dir.path().join("delete.html")).unwrap();
        std::fs::write(dir.path().join("new.html"), "fresh").unwrap();

        let diff = compute_diff(dir.path()).unwrap();
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.unchanged, 1);
    }
}
