//! Asset processing for Mythic: images, CSS, JavaScript, and Sass/SCSS.

pub mod images;
pub mod sass;
pub mod scripts;
pub mod styles;

use anyhow::Result;
use mythic_core::config::SiteConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Processed asset paths available to templates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    pub css_path: Option<String>,
    pub js_path: Option<String>,
    /// SRI integrity hash for CSS (sha384).
    pub css_integrity: Option<String>,
    /// SRI integrity hash for JS (sha384).
    pub js_integrity: Option<String>,
}

/// Compute a SHA-384 SRI hash for the given content.
pub fn compute_sri(content: &str) -> String {
    use base64::Engine;
    use sha2::{Digest, Sha384};

    let mut hasher = Sha384::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    let b64 = base64::engine::general_purpose::STANDARD.encode(hash);
    format!("sha384-{b64}")
}

/// Run the full asset pipeline and return the manifest plus any warnings.
pub fn process_assets(config: &SiteConfig, root: &Path) -> Result<(AssetManifest, Vec<String>)> {
    let output_dir = root.join(&config.output_dir);
    let mut manifest = AssetManifest::default();
    let mut warnings = Vec::new();

    // Process images
    let static_dir = root.join(&config.static_dir);
    if static_dir.exists() {
        let (_, image_warnings) = images::process_images(config, root)?;
        warnings.extend(image_warnings);
    }

    // Compile Sass if configured
    let styles_dir = root.join(&config.styles_dir);
    if styles_dir.exists() {
        let sass_enabled = config.sass.as_ref().map(|s| s.enabled).unwrap_or(true);

        let css = if sass_enabled {
            sass::compile_and_concat(&styles_dir)?
        } else {
            styles::concat_css(&styles_dir)?
        };

        if !css.is_empty() {
            let minified = styles::minify_css(&css);
            manifest.css_integrity = Some(compute_sri(&minified));
            let path = styles::write_hashed(&minified, &output_dir)?;
            manifest.css_path = Some(format!("/{path}"));
        }
    }

    // Process JS
    let scripts_dir = root.join(&config.scripts_dir);
    if scripts_dir.exists() {
        let js = scripts::concat_js(&scripts_dir)?;
        if !js.is_empty() {
            let minified = scripts::minify_js(&js);
            manifest.js_integrity = Some(compute_sri(&minified));
            let path = scripts::write_hashed(&minified, &output_dir)?;
            manifest.js_path = Some(format!("/{path}"));
        }
    }

    Ok((manifest, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sri_hash_is_deterministic() {
        let h1 = compute_sri("body { color: red; }");
        let h2 = compute_sri("body { color: red; }");
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha384-"));
    }

    #[test]
    fn sri_hash_changes_with_content() {
        let h1 = compute_sri("body { color: red; }");
        let h2 = compute_sri("body { color: blue; }");
        assert_ne!(h1, h2);
    }
}
