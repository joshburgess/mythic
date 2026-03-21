//! Asset processing for Mythic: images, CSS, JavaScript, and Sass/SCSS.

pub mod images;
pub mod styles;
pub mod scripts;
pub mod sass;

use anyhow::Result;
use mythic_core::config::SiteConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Processed asset paths available to templates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    pub css_path: Option<String>,
    pub js_path: Option<String>,
}

/// Run the full asset pipeline and return the manifest.
pub fn process_assets(config: &SiteConfig, root: &Path) -> Result<AssetManifest> {
    let output_dir = root.join(&config.output_dir);
    let mut manifest = AssetManifest::default();

    // Process images
    let static_dir = root.join(&config.static_dir);
    if static_dir.exists() {
        images::process_images(config, root)?;
    }

    // Compile Sass if configured
    let styles_dir = root.join(&config.styles_dir);
    if styles_dir.exists() {
        let sass_enabled = config
            .sass
            .as_ref()
            .map(|s| s.enabled)
            .unwrap_or(true);

        let css = if sass_enabled {
            sass::compile_and_concat(&styles_dir)?
        } else {
            styles::concat_css(&styles_dir)?
        };

        if !css.is_empty() {
            let minified = styles::minify_css(&css);
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
            let path = scripts::write_hashed(&minified, &output_dir)?;
            manifest.js_path = Some(format!("/{path}"));
        }
    }

    Ok(manifest)
}
