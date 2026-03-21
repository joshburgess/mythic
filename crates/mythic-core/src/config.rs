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
