//! Page representation used throughout the build pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A single content page flowing through the build pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Path to the source file.
    pub source_path: PathBuf,
    /// URL slug derived from the relative path.
    pub slug: String,
    /// Parsed frontmatter.
    pub frontmatter: Frontmatter,
    /// Raw markdown content (after frontmatter is stripped).
    pub raw_content: String,
    /// Rendered HTML (populated during the render stage).
    pub rendered_html: Option<String>,
    /// Output file path (populated during the output stage).
    pub output_path: Option<PathBuf>,
    /// Content hash for incremental builds.
    pub content_hash: u64,
}

/// Parsed frontmatter from a content file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub draft: Option<bool>,
    #[serde(default = "default_layout")]
    pub layout: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub extra: Option<HashMap<String, serde_json::Value>>,
}

fn default_layout() -> Option<String> {
    Some("default".to_string())
}
