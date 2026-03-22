//! Image processing: resize, convert to WebP, and content-hash filenames.

use anyhow::{Context, Result};
use image::ImageFormat;
use mythic_core::config::SiteConfig;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_BREAKPOINTS: &[u32] = &[400, 800, 1200];

/// A single generated image variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub path: String,
    pub width: u32,
    pub format: String,
}

/// Mapping from original image paths to their generated variants.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageManifest {
    pub images: HashMap<String, Vec<GeneratedImage>>,
}

/// Process all images: generate resized WebP variants with content-hashed filenames.
pub fn process_images(config: &SiteConfig, root: &Path) -> Result<ImageManifest> {
    let static_dir = root.join(&config.static_dir);
    let output_base = root.join(&config.output_dir).join("assets/img");
    let breakpoints = config
        .image_breakpoints
        .as_deref()
        .unwrap_or(DEFAULT_BREAKPOINTS);

    let image_files = discover_images(&static_dir);
    if image_files.is_empty() {
        return Ok(ImageManifest::default());
    }

    std::fs::create_dir_all(&output_base).context("Failed to create image output directory")?;

    let results: Vec<(String, Vec<GeneratedImage>)> = image_files
        .par_iter()
        .filter_map(|path| {
            match process_single_image(path, &static_dir, &output_base, breakpoints) {
                Ok(variants) => {
                    let rel = path
                        .strip_prefix(&static_dir)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    Some((rel, variants))
                }
                Err(e) => {
                    eprintln!("  Warning: failed to process {}: {e}", path.display());
                    None
                }
            }
        })
        .collect();

    let mut manifest = ImageManifest::default();
    for (key, variants) in results {
        manifest.images.insert(key, variants);
    }

    Ok(manifest)
}

fn discover_images(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            matches!(
                e.path().extension().and_then(|x| x.to_str()),
                Some("jpg" | "jpeg" | "png" | "gif" | "webp")
            )
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn process_single_image(
    path: &Path,
    _static_dir: &Path,
    output_base: &Path,
    breakpoints: &[u32],
) -> Result<Vec<GeneratedImage>> {
    let data = std::fs::read(path)?;
    let content_hash = {
        let mut h = DefaultHasher::new();
        data.hash(&mut h);
        h.finish()
    };
    let hash_str = format!("{:x}", content_hash);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();

    let img = image::load_from_memory(&data)
        .with_context(|| format!("Failed to decode image: {}", path.display()))?;

    let original_width = img.width();
    let mut variants = Vec::new();

    // Copy original
    let orig_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let orig_name = format!("{stem}-{hash_str}.{orig_ext}");
    let orig_dest = output_base.join(&orig_name);
    if !orig_dest.exists() {
        std::fs::copy(path, &orig_dest)?;
    }
    variants.push(GeneratedImage {
        path: format!("assets/img/{orig_name}"),
        width: original_width,
        format: orig_ext.to_string(),
    });

    // Generate WebP at each breakpoint
    for &bp in breakpoints {
        if bp >= original_width {
            continue;
        }
        let resized = img.resize(bp, u32::MAX, image::imageops::FilterType::Lanczos3);
        let webp_name = format!("{stem}-{hash_str}-{bp}.webp");
        let webp_dest = output_base.join(&webp_name);
        if !webp_dest.exists() {
            resized.save_with_format(&webp_dest, ImageFormat::WebP)?;
        }
        variants.push(GeneratedImage {
            path: format!("assets/img/{webp_name}"),
            width: bp,
            format: "webp".to_string(),
        });
    }

    // Full-size WebP
    let webp_full_name = format!("{stem}-{hash_str}.webp");
    let webp_full_dest = output_base.join(&webp_full_name);
    if !webp_full_dest.exists() {
        img.save_with_format(&webp_full_dest, ImageFormat::WebP)?;
    }
    variants.push(GeneratedImage {
        path: format!("assets/img/{webp_full_name}"),
        width: original_width,
        format: "webp".to_string(),
    });

    Ok(variants)
}

/// Generate a responsive `<picture>` element from the manifest.
pub fn picture_tag(
    manifest: &ImageManifest,
    src: &str,
    alt: &str,
    sizes: Option<&str>,
) -> Result<String> {
    let variants = manifest
        .images
        .get(src)
        .with_context(|| format!("Image not found in manifest: {src}"))?;

    let sizes_attr = sizes.unwrap_or("100vw");

    let webp_sources: Vec<&GeneratedImage> =
        variants.iter().filter(|v| v.format == "webp").collect();

    let original = variants
        .iter()
        .find(|v| v.format != "webp")
        .with_context(|| format!("No original found for: {src}"))?;

    let mut html = String::from("<picture>\n");

    if !webp_sources.is_empty() {
        let srcset: Vec<String> = webp_sources
            .iter()
            .map(|v| format!("/{} {}w", v.path, v.width))
            .collect();
        html.push_str(&format!(
            "  <source type=\"image/webp\" srcset=\"{}\" sizes=\"{sizes_attr}\">\n",
            srcset.join(", ")
        ));
    }

    html.push_str(&format!(
        "  <img src=\"/{}\" alt=\"{alt}\" width=\"{}\" height=\"auto\" loading=\"lazy\">\n",
        original.path, original.width
    ));
    html.push_str("</picture>");

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_png(dir: &Path, name: &str, width: u32, height: u32) {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        img.save(dir.join(name)).unwrap();
    }

    #[test]
    fn process_png_image() {
        let dir = tempfile::tempdir().unwrap();
        let static_dir = dir.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();
        create_test_png(&static_dir, "test.png", 1600, 1200);

        let config = SiteConfig::for_testing("Test", "http://localhost");
        let manifest = process_images(&config, dir.path()).unwrap();
        let variants = manifest.images.get("test.png").unwrap();

        // Should have: original PNG + WebP at 400, 800, 1200 + full WebP
        assert!(variants.len() >= 4);
        assert!(variants.iter().any(|v| v.format == "png"));
        assert!(variants.iter().any(|v| v.format == "webp"));
    }

    #[test]
    fn process_jpeg_image() {
        let dir = tempfile::tempdir().unwrap();
        let static_dir = dir.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();

        let img = image::RgbImage::from_fn(1600, 1200, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 64])
        });
        img.save(static_dir.join("photo.jpg")).unwrap();

        let config = SiteConfig::for_testing("Test", "http://localhost");
        let manifest = process_images(&config, dir.path()).unwrap();
        let variants = manifest.images.get("photo.jpg").unwrap();

        assert!(variants.iter().any(|v| v.format == "webp"));
        assert!(variants.iter().any(|v| v.format == "jpg"));
    }

    #[test]
    fn skip_unchanged_images() {
        let dir = tempfile::tempdir().unwrap();
        let static_dir = dir.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();
        create_test_png(&static_dir, "test.png", 1600, 1200);

        let config = SiteConfig::for_testing("Test", "http://localhost");
        process_images(&config, dir.path()).unwrap();

        // Second run should reuse existing files (they exist check)
        let manifest = process_images(&config, dir.path()).unwrap();
        assert!(!manifest.images.is_empty());
    }

    #[test]
    fn picture_tag_basic() {
        let mut manifest = ImageManifest::default();
        manifest.images.insert(
            "photo.jpg".to_string(),
            vec![
                GeneratedImage {
                    path: "assets/img/photo-abc.jpg".to_string(),
                    width: 1600,
                    format: "jpg".to_string(),
                },
                GeneratedImage {
                    path: "assets/img/photo-abc-800.webp".to_string(),
                    width: 800,
                    format: "webp".to_string(),
                },
                GeneratedImage {
                    path: "assets/img/photo-abc.webp".to_string(),
                    width: 1600,
                    format: "webp".to_string(),
                },
            ],
        );

        let html = picture_tag(&manifest, "photo.jpg", "A photo", None).unwrap();
        assert!(html.contains("<picture>"));
        assert!(html.contains("image/webp"));
        assert!(html.contains("loading=\"lazy\""));
        assert!(html.contains("alt=\"A photo\""));
    }

    #[test]
    fn picture_tag_missing_image_errors() {
        let manifest = ImageManifest::default();
        assert!(picture_tag(&manifest, "missing.jpg", "alt", None).is_err());
    }

    use mythic_core::config::SiteConfig;
}
