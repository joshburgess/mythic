//! Hugo to Mythic migration.

use anyhow::{Context, Result};
use std::path::Path;
use walkdir::WalkDir;

use super::convert::go_template_to_tera;
use super::MigrationReport;

/// Migrate a Hugo site to Mythic format.
pub fn migrate(source: &Path, output: &Path) -> Result<MigrationReport> {
    let mut report = MigrationReport::default();

    std::fs::create_dir_all(output)?;

    // 1. Convert config
    convert_config(source, output, &mut report)?;

    // 2. Copy content
    migrate_content(source, output, &mut report)?;

    // 3. Convert layouts
    migrate_layouts(source, output, &mut report)?;

    // 4. Copy static
    copy_dir_if_exists(&source.join("static"), &output.join("static"), &mut report)?;

    // 5. Copy data
    copy_dir_if_exists(&source.join("data"), &output.join("_data"), &mut report)?;

    // 6. Convert shortcodes
    migrate_shortcodes(source, output, &mut report)?;

    Ok(report)
}

fn convert_config(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    // Try different Hugo config files
    let config_candidates = [
        ("config.toml", ConfigFormat::Toml),
        ("hugo.toml", ConfigFormat::Toml),
        ("config.yaml", ConfigFormat::Yaml),
        ("hugo.yaml", ConfigFormat::Yaml),
        ("config.json", ConfigFormat::Json),
    ];

    for (filename, format) in &config_candidates {
        let path = source.join(filename);
        if path.exists() {
            return convert_config_file(&path, format, output, report);
        }
    }

    report.warn("No Hugo config file found");
    std::fs::write(
        output.join("mythic.toml"),
        "title = \"Migrated Site\"\nbase_url = \"http://localhost:3000\"\n",
    )?;
    Ok(())
}

enum ConfigFormat {
    Toml,
    Yaml,
    Json,
}

fn convert_config_file(
    path: &Path,
    format: &ConfigFormat,
    output: &Path,
    report: &mut MigrationReport,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;

    let (title, base_url) = match format {
        ConfigFormat::Toml => {
            let val: toml::Value =
                toml::from_str(&content).context("Failed to parse Hugo TOML config")?;
            (
                val.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Migrated Site")
                    .to_string(),
                val.get("baseURL")
                    .and_then(|v| v.as_str())
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            )
        }
        ConfigFormat::Yaml => {
            let val: serde_yaml::Value =
                serde_yaml::from_str(&content).context("Failed to parse Hugo YAML config")?;
            (
                val["title"].as_str().unwrap_or("Migrated Site").to_string(),
                val["baseURL"]
                    .as_str()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            )
        }
        ConfigFormat::Json => {
            let val: serde_json::Value =
                serde_json::from_str(&content).context("Failed to parse Hugo JSON config")?;
            (
                val["title"].as_str().unwrap_or("Migrated Site").to_string(),
                val["baseURL"]
                    .as_str()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            )
        }
    };

    let title = escape_toml_string(&title);
    let base_url = escape_toml_string(&base_url);
    let toml = format!("title = \"{title}\"\nbase_url = \"{base_url}\"\n");
    std::fs::write(output.join("mythic.toml"), toml)?;
    report.files_converted += 1;

    Ok(())
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn migrate_content(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let content_dir = source.join("content");
    if !content_dir.exists() {
        return Ok(());
    }

    let out_content = output.join("content");

    for entry in WalkDir::new(&content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let rel = path.strip_prefix(&content_dir).unwrap_or(path);
        let target = out_content.join(rel);

        if path.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "md" || ext == "markdown" {
            let content = std::fs::read_to_string(path)?;
            let converted = convert_hugo_frontmatter(&content);
            std::fs::write(&target, converted)?;
            report.files_converted += 1;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, &target)?;
            report.files_copied += 1;
        }
    }

    Ok(())
}

fn convert_hugo_frontmatter(content: &str) -> String {
    // Hugo uses type, weight, etc. — move non-standard fields to extra
    // For now, just pass through; Hugo YAML/TOML frontmatter is mostly compatible
    content.to_string()
}

fn migrate_layouts(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let layouts_dir = source.join("layouts");
    if !layouts_dir.exists() {
        return Ok(());
    }

    let out_templates = output.join("templates");

    for entry in WalkDir::new(&layouts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let rel = path.strip_prefix(&layouts_dir).unwrap_or(path);
        let target = out_templates.join(rel);

        if path.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        // Skip shortcodes — handled separately
        if rel.starts_with("shortcodes") {
            continue;
        }

        let content = std::fs::read_to_string(path)?;
        let (converted, warnings) = go_template_to_tera(&content);

        for w in warnings {
            report.warn(format!("{}: {w}", rel.display()));
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, converted)?;
        report.files_converted += 1;
    }

    Ok(())
}

fn migrate_shortcodes(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let shortcodes_dir = source.join("layouts/shortcodes");
    if !shortcodes_dir.exists() {
        return Ok(());
    }

    let out_shortcodes = output.join("shortcodes");
    std::fs::create_dir_all(&out_shortcodes)?;

    for entry in std::fs::read_dir(&shortcodes_dir)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let (converted, warnings) = go_template_to_tera(&content);

        // Hugo shortcodes use {{ .Get "param" }} → {{ param }}
        let converted = converted
            .replace("{{ .Get \"", "{{ ")
            .replace("\" }}", " }}");

        // {{ .Inner }} → {{ inner }}
        let converted = converted.replace("{{ .Inner }}", "{{ inner }}");
        let converted = converted.replace("{{.Inner}}", "{{ inner }}");

        for w in warnings {
            report.warn(format!(
                "shortcodes/{}: {w}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }

        if let Some(fname) = path.file_name() {
            std::fs::write(out_shortcodes.join(fname), converted)?;
        }
        report.files_converted += 1;
    }

    Ok(())
}

fn copy_dir_if_exists(src: &Path, dest: &Path, report: &mut MigrationReport) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap_or(path);
        let target = dest.join(rel);

        if path.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, &target)?;
            report.files_copied += 1;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_toml_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(
            src.path().join("config.toml"),
            "title = \"Hugo Site\"\nbaseURL = \"https://example.com\"",
        )
        .unwrap();

        let report = migrate(src.path(), out.path()).unwrap();
        assert!(report.files_converted > 0);

        let config = std::fs::read_to_string(out.path().join("mythic.toml")).unwrap();
        assert!(config.contains("Hugo Site"));
        assert!(config.contains("https://example.com"));
    }

    #[test]
    fn config_yaml_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(
            src.path().join("config.yaml"),
            "title: Hugo YAML Site\nbaseURL: https://yaml.example.com",
        )
        .unwrap();

        let report = migrate(src.path(), out.path()).unwrap();
        let config = std::fs::read_to_string(out.path().join("mythic.toml")).unwrap();
        assert!(config.contains("Hugo YAML Site"));
    }

    #[test]
    fn content_migration() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join("config.toml"), "title = \"T\"").unwrap();
        let content = src.path().join("content/posts");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("hello.md"),
            "---\ntitle: Hello\n---\n# Hello World",
        )
        .unwrap();

        migrate(src.path(), out.path()).unwrap();

        let migrated = out.path().join("content/posts/hello.md");
        assert!(migrated.exists());
    }

    #[test]
    fn template_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join("config.toml"), "title = \"T\"").unwrap();
        let layouts = src.path().join("layouts");
        std::fs::create_dir_all(&layouts).unwrap();
        std::fs::write(
            layouts.join("baseof.html"),
            "<html><body>{{ .Content }}</body></html>",
        )
        .unwrap();

        migrate(src.path(), out.path()).unwrap();

        let template = std::fs::read_to_string(out.path().join("templates/baseof.html")).unwrap();
        assert!(template.contains("{{ content | safe }}"));
    }

    #[test]
    fn shortcode_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join("config.toml"), "title = \"T\"").unwrap();
        let sc = src.path().join("layouts/shortcodes");
        std::fs::create_dir_all(&sc).unwrap();
        std::fs::write(
            sc.join("youtube.html"),
            "<iframe src=\"https://youtube.com/embed/{{ .Get \"id\" }}\"></iframe>",
        )
        .unwrap();

        migrate(src.path(), out.path()).unwrap();

        let shortcode =
            std::fs::read_to_string(out.path().join("shortcodes/youtube.html")).unwrap();
        assert!(shortcode.contains("{{ id"));
        assert!(!shortcode.contains(".Get"));
    }

    #[test]
    fn titles_with_double_quotes_properly_escaped_in_toml() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(
            src.path().join("config.toml"),
            "title = \"Hugo's \\\"Best\\\" Site\"\nbaseURL = \"https://example.com\"",
        )
        .unwrap();

        migrate(src.path(), out.path()).unwrap();
        let config = std::fs::read_to_string(out.path().join("mythic.toml")).unwrap();

        // The generated TOML should be valid (double quotes escaped)
        let parsed: Result<toml::Value, _> = config.parse();
        assert!(
            parsed.is_ok(),
            "Generated mythic.toml should be valid TOML, got: {config}"
        );
    }

    #[test]
    fn escape_toml_string_handles_quotes_and_backslashes() {
        assert_eq!(escape_toml_string(r#"hello"world"#), r#"hello\"world"#);
        assert_eq!(escape_toml_string(r"path\to\file"), r"path\\to\\file");
        assert_eq!(
            escape_toml_string(r#"say "hi" to\me"#),
            r#"say \"hi\" to\\me"#
        );
    }
}
