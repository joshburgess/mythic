//! Page representation used throughout the build pipeline.
//!
//! A [`Page`] flows through the pipeline stages: discovery → frontmatter parsing →
//! markdown rendering → template application → file output. Fields are progressively
//! populated at each stage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A single content page flowing through the build pipeline.
///
/// Created during content discovery with `source_path`, `slug`, `frontmatter`,
/// `raw_content`, and `content_hash` populated. The `rendered_html` and `toc`
/// fields are filled during the markdown rendering stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Absolute path to the source markdown file.
    pub source_path: PathBuf,
    /// URL slug derived from the file's path relative to the content directory.
    /// Example: `content/blog/my-post.md` → `blog/my-post`.
    pub slug: String,
    /// Parsed frontmatter metadata.
    pub frontmatter: Frontmatter,
    /// Raw markdown content (everything after the frontmatter delimiters).
    pub raw_content: String,
    /// Rendered HTML, populated during the markdown rendering stage.
    /// After template application, this contains the final full-page HTML.
    pub rendered_html: Option<String>,
    /// Destination file path, populated during the output stage.
    pub output_path: Option<PathBuf>,
    /// Hash of the raw file content, used for incremental build caching.
    /// If this matches the cached hash, the page is skipped during output.
    pub content_hash: u64,
    /// Table of contents extracted from headings during markdown rendering.
    /// Available in templates as `{{ toc }}`.
    #[serde(default)]
    pub toc: Vec<TocEntry>,
}

/// A single heading entry in the table of contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// Heading level (1-6, corresponding to h1-h6).
    pub level: u32,
    /// Plain text content of the heading (HTML tags stripped).
    pub text: String,
    /// Slugified anchor ID, added as an `id` attribute to the heading element.
    pub id: String,
}

/// Parsed frontmatter from a content file.
///
/// Supports both YAML (`---` delimiters) and TOML (`+++` delimiters).
/// Only `title` is required; all other fields are optional with sensible defaults.
///
/// # Example (YAML)
///
/// ```yaml
/// ---
/// title: "My Post"
/// date: "2024-01-15"
/// draft: false
/// layout: blog
/// tags:
///   - rust
///   - web
/// extra:
///   author: "Alice"
/// ---
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Page title (required).
    pub title: String,
    /// Publication date as a string (e.g., `"2024-01-15"`).
    #[serde(default)]
    pub date: Option<String>,
    /// If `true`, the page is excluded from builds (unless `--drafts` is passed).
    #[serde(default)]
    pub draft: Option<bool>,
    /// Template layout name. Defaults to `"default"`.
    /// Maps to `{layout}.html` (Tera) or `{layout}.hbs` (Handlebars).
    #[serde(default = "default_layout")]
    pub layout: Option<String>,
    /// Tag list for taxonomy classification.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Arbitrary key-value metadata, accessible in templates as `{{ page.extra.key }}`.
    #[serde(default)]
    pub extra: Option<HashMap<String, serde_json::Value>>,
    /// If `false`, excludes the page from `sitemap.xml`.
    #[serde(default)]
    pub sitemap: Option<bool>,
    /// Locale code for i18n (e.g., `"en"`, `"es"`).
    /// Can also be inferred from the content directory structure.
    #[serde(default)]
    pub locale: Option<String>,
}

fn default_layout() -> Option<String> {
    Some("default".to_string())
}
