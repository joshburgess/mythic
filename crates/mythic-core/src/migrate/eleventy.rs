//! Eleventy to Mythic migration.

use anyhow::{Context, Result};
use std::path::Path;
use walkdir::WalkDir;

use super::convert::nunjucks_to_tera;
use super::MigrationReport;

/// Migrate an Eleventy site to Mythic format.
pub fn migrate(source: &Path, output: &Path) -> Result<MigrationReport> {
    let mut report = MigrationReport::default();

    std::fs::create_dir_all(output)?;

    // 1. Extract config from .eleventy.js or eleventy.config.js
    extract_config(source, output, &mut report)?;

    // 2. Copy/convert content
    migrate_content(source, output, &mut report)?;

    // 3. Copy includes → templates
    migrate_includes(source, output, &mut report)?;

    // 4. Copy/convert data
    migrate_data(source, output, &mut report)?;

    Ok(report)
}

fn extract_config(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let config_candidates = [
        ".eleventy.js",
        "eleventy.config.js",
        "eleventy.config.cjs",
        "eleventy.config.mjs",
    ];

    let mut input_dir = "src".to_string();
    let mut data_dir = "_data".to_string();

    for candidate in &config_candidates {
        let path = source.join(candidate);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;

            // Extract basic settings from JS config via regex-like scanning
            if let Some(dir) = extract_js_string(&content, "input") {
                input_dir = dir;
            }
            if let Some(dir) = extract_js_string(&content, "data") {
                data_dir = dir;
            }

            report.warn(format!(
                "{candidate} parsed for basic settings. Complex JS config needs manual review."
            ));
            break;
        }
    }

    let toml = format!(
        "title = \"Migrated Site\"\nbase_url = \"http://localhost:3000\"\ncontent_dir = \"{input_dir}\"\ndata_dir = \"{data_dir}\"\n"
    );
    std::fs::write(output.join("mythic.toml"), toml)?;
    report.files_converted += 1;

    Ok(())
}

fn extract_js_string(content: &str, key: &str) -> Option<String> {
    // Look for patterns like: input: "src" or input: 'src'
    let patterns = [
        format!("{key}: \""),
        format!("{key}: '"),
        format!("{key}:\""),
        format!("{key}:'"),
    ];

    for pattern in &patterns {
        if let Some(start) = content.find(pattern.as_str()) {
            let after = &content[start + pattern.len()..];
            let quote = if pattern.ends_with('"') { '"' } else { '\'' };
            if let Some(end) = after.find(quote) {
                return Some(after[..end].to_string());
            }
        }
    }

    None
}

fn migrate_content(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    // Eleventy can use various input dirs; try common ones
    let content_dirs = ["src", ".", "content"];

    for dir_name in &content_dirs {
        let content_dir = source.join(dir_name);
        if !content_dir.exists() {
            continue;
        }

        let out_content = output.join("content");

        for entry in WalkDir::new(&content_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let rel = path.strip_prefix(&content_dir).unwrap_or(path);

            // Skip node_modules, config files, etc.
            if rel.starts_with("node_modules") || rel.starts_with("_") || rel.starts_with(".") {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let target = out_content.join(rel);

            match ext {
                "md" | "markdown" => {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(path, &target)?;
                    report.files_copied += 1;
                }
                "njk" | "nunjucks" => {
                    // Convert .njk → .md or keep as template depending on context
                    let content = std::fs::read_to_string(path)?;
                    let (converted, warnings) = nunjucks_to_tera(&content);

                    for w in warnings {
                        report.warn(format!("{}: {w}", rel.display()));
                    }

                    let target = target.with_extension("html");
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&target, converted)?;
                    report.files_converted += 1;
                }
                "liquid" => {
                    let content = std::fs::read_to_string(path)?;
                    let (converted, warnings) = super::convert::liquid_to_tera(&content);

                    for w in warnings {
                        report.warn(format!("{}: {w}", rel.display()));
                    }

                    let target = target.with_extension("html");
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&target, converted)?;
                    report.files_converted += 1;
                }
                _ => {}
            }
        }

        break; // Only process the first existing content dir
    }

    Ok(())
}

fn migrate_includes(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let includes_dir = source.join("_includes");
    if !includes_dir.exists() {
        // Try src/_includes
        let alt = source.join("src/_includes");
        if !alt.exists() {
            return Ok(());
        }
        return migrate_includes_from(&alt, output, report);
    }

    migrate_includes_from(&includes_dir, output, report)
}

fn migrate_includes_from(
    includes_dir: &Path,
    output: &Path,
    report: &mut MigrationReport,
) -> Result<()> {
    let out_templates = output.join("templates");

    for entry in WalkDir::new(includes_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let rel = path.strip_prefix(includes_dir).unwrap_or(path);
        let target = out_templates.join(rel);

        if path.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content = std::fs::read_to_string(path)?;

        let converted = match ext {
            "njk" | "nunjucks" => {
                let (c, warnings) = nunjucks_to_tera(&content);
                for w in warnings {
                    report.warn(format!("_includes/{}: {w}", rel.display()));
                }
                c
            }
            "liquid" => {
                let (c, warnings) = super::convert::liquid_to_tera(&content);
                for w in warnings {
                    report.warn(format!("_includes/{}: {w}", rel.display()));
                }
                c
            }
            _ => content,
        };

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, converted)?;
        report.files_converted += 1;
    }

    Ok(())
}

fn migrate_data(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let data_dirs = [source.join("_data"), source.join("src/_data")];

    for data_dir in &data_dirs {
        if !data_dir.exists() {
            continue;
        }

        let out_data = output.join("_data");

        for entry in WalkDir::new(data_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let rel = path.strip_prefix(data_dir).unwrap_or(path);
            let target = out_data.join(rel);

            if path.is_dir() {
                std::fs::create_dir_all(&target)?;
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            match ext {
                "json" | "yaml" | "yml" => {
                    // Check for Eleventy directory data files (dirname.json)
                    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
                    let parent_name = path
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    if name == parent_name {
                        // This is a directory data file — convert to _dir.yaml
                        let content = std::fs::read_to_string(path)?;
                        let dir_target = target.parent().unwrap().join("_dir.yaml");
                        if let Some(parent) = dir_target.parent() {
                            std::fs::create_dir_all(parent)?;
                        }

                        if ext == "json" {
                            let val: serde_json::Value = serde_json::from_str(&content)
                                .with_context(|| format!("Invalid JSON: {}", path.display()))?;
                            let yaml = serde_yaml::to_string(&val)?;
                            std::fs::write(&dir_target, yaml)?;
                        } else {
                            std::fs::write(&dir_target, content)?;
                        }
                        report.files_converted += 1;
                    } else {
                        if let Some(parent) = target.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::copy(path, &target)?;
                        report.files_copied += 1;
                    }
                }
                "js" | "cjs" | "mjs" => {
                    report.warn(format!(
                        "JS data file needs manual conversion: {}",
                        rel.display()
                    ));
                }
                _ => {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(path, &target)?;
                    report.files_copied += 1;
                }
            }
        }

        break;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_extraction() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(
            src.path().join(".eleventy.js"),
            r#"module.exports = function(config) {
                return { dir: { input: "src", data: "_data" } };
            };"#,
        )
        .unwrap();

        // Create src dir so content migration doesn't fail
        std::fs::create_dir_all(src.path().join("src")).unwrap();

        let report = migrate(src.path(), out.path()).unwrap();
        assert!(report.files_converted > 0);

        let config = std::fs::read_to_string(out.path().join("mythic.toml")).unwrap();
        assert!(config.contains("content_dir = \"src\""));
    }

    #[test]
    fn nunjucks_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        let content = src.path().join("src");
        std::fs::create_dir_all(&content).unwrap();

        std::fs::write(
            src.path().join(".eleventy.js"),
            "module.exports = { dir: { input: \"src\" } };",
        )
        .unwrap();

        std::fs::write(
            content.join("page.njk"),
            "{% for item in items %}<li>{{ item | dump }}</li>{% endfor %}",
        )
        .unwrap();

        migrate(src.path(), out.path()).unwrap();

        let converted = std::fs::read_to_string(out.path().join("content/page.html")).unwrap();
        assert!(converted.contains("json_encode()"));
    }

    #[test]
    fn directory_data_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join(".eleventy.js"), "module.exports = {};").unwrap();

        let data = src.path().join("_data/posts");
        std::fs::create_dir_all(&data).unwrap();

        // Directory data file: posts/posts.json
        std::fs::write(
            data.join("posts.json"),
            r#"{"layout": "post", "permalink": "/blog/{{ slug }}/"}"#,
        )
        .unwrap();

        migrate(src.path(), out.path()).unwrap();

        let dir_yaml = out.path().join("_data/posts/_dir.yaml");
        assert!(dir_yaml.exists());
    }

    #[test]
    fn js_data_files_flagged() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join(".eleventy.js"), "module.exports = {};").unwrap();

        let data = src.path().join("_data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("api.js"), "module.exports = async () => {}").unwrap();

        let report = migrate(src.path(), out.path()).unwrap();
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("api.js") && w.contains("manual")));
    }
}
