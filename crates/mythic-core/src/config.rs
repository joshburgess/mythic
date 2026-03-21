//! Site configuration loaded from `mythic.toml`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level site configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub title: String,
    pub base_url: String,
    #[serde(default = "default_content_dir")]
    pub content_dir: PathBuf,
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    #[serde(default = "default_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "default_static_dir")]
    pub static_dir: PathBuf,
    #[serde(default = "default_styles_dir")]
    pub styles_dir: PathBuf,
    #[serde(default = "default_scripts_dir")]
    pub scripts_dir: PathBuf,
    #[serde(default)]
    pub image_breakpoints: Option<Vec<u32>>,
    #[serde(default)]
    pub sass: Option<SassConfig>,
    #[serde(default)]
    pub taxonomies: Vec<TaxonomyConfig>,
    #[serde(default)]
    pub feed: Option<FeedConfig>,
    #[serde(default)]
    pub highlight: Option<HighlightConfig>,
    #[serde(default)]
    pub toc: Option<TocConfig>,
}

/// Sass/SCSS compilation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SassConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Taxonomy definition in config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyConfig {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub feed: bool,
}

/// Feed (Atom/RSS) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    #[serde(default = "default_feed_title")]
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default = "default_feed_entries")]
    pub entries: usize,
}

fn default_feed_title() -> String {
    "Feed".to_string()
}

fn default_feed_entries() -> usize {
    20
}

/// Syntax highlighting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub line_numbers: bool,
}

fn default_theme() -> String {
    "base16-ocean.dark".to_string()
}

/// Table of contents configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocConfig {
    #[serde(default = "default_min_level")]
    pub min_level: u32,
    #[serde(default = "default_max_level")]
    pub max_level: u32,
}

fn default_min_level() -> u32 {
    2
}

fn default_max_level() -> u32 {
    4
}

fn default_content_dir() -> PathBuf {
    PathBuf::from("content")
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("public")
}

fn default_template_dir() -> PathBuf {
    PathBuf::from("templates")
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("_data")
}

fn default_static_dir() -> PathBuf {
    PathBuf::from("static")
}

fn default_styles_dir() -> PathBuf {
    PathBuf::from("styles")
}

fn default_scripts_dir() -> PathBuf {
    PathBuf::from("scripts")
}

impl SiteConfig {
    /// Create a config suitable for tests.
    pub fn for_testing(title: &str, base_url: &str) -> Self {
        SiteConfig {
            title: title.to_string(),
            base_url: base_url.to_string(),
            content_dir: default_content_dir(),
            output_dir: default_output_dir(),
            template_dir: default_template_dir(),
            data_dir: default_data_dir(),
            static_dir: default_static_dir(),
            styles_dir: default_styles_dir(),
            scripts_dir: default_scripts_dir(),
            image_breakpoints: None,
            sass: None,
            taxonomies: Vec::new(),
            feed: None,
            highlight: None,
            toc: None,
        }
    }
}

/// Load site configuration from a TOML file.
pub fn load_config(path: &Path) -> Result<SiteConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: SiteConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    #[test]
    fn load_valid_config() {
        let config = load_config(Path::new("../../fixtures/basic-site/mythic.toml")).unwrap();
        assert_eq!(config.title, "Basic Test Site");
        assert_eq!(config.base_url, "http://localhost:3000");
    }

    #[test]
    fn missing_optional_fields_get_defaults() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "title = \"Minimal\"\nbase_url = \"http://example.com\"\n").unwrap();
        let config = load_config(f.path()).unwrap();
        assert_eq!(config.content_dir, PathBuf::from("content"));
        assert_eq!(config.output_dir, PathBuf::from("public"));
        assert_eq!(config.template_dir, PathBuf::from("templates"));
        assert_eq!(config.data_dir, PathBuf::from("_data"));
    }

    #[test]
    fn invalid_toml_returns_error() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "not valid {{{{ toml").unwrap();
        assert!(load_config(f.path()).is_err());
    }

    #[test]
    fn missing_file_returns_error() {
        assert!(load_config(Path::new("/nonexistent/mythic.toml")).is_err());
    }
}
