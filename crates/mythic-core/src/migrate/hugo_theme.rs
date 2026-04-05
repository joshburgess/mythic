//! Hugo theme to Mythic starter converter.
//!
//! Converts a Hugo theme directory into a Mythic starter template,
//! translating Go templates to Tera and mapping Hugo conventions
//! to Mythic equivalents.

use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

use super::convert::go_template_to_tera;
use super::MigrationReport;

/// Convert a Hugo theme to a Mythic starter template.
pub fn convert_theme(theme_dir: &Path, output: &Path) -> Result<MigrationReport> {
    let mut report = MigrationReport::default();

    std::fs::create_dir_all(output)?;

    // 1. Read theme metadata
    convert_theme_config(theme_dir, output, &mut report)?;

    // 2. Convert layouts → templates
    convert_layouts(theme_dir, output, &mut report)?;

    // 3. Copy and convert assets
    convert_assets(theme_dir, output, &mut report)?;

    // 4. Copy static files
    copy_dir(
        &theme_dir.join("static"),
        &output.join("static"),
        &mut report,
    )?;

    // 5. Convert shortcodes
    convert_shortcodes(theme_dir, output, &mut report)?;

    // 6. Convert archetypes → content scaffolds
    convert_archetypes(theme_dir, output, &mut report)?;

    // 7. Create example content from exampleSite if present
    convert_example_site(theme_dir, output, &mut report)?;

    // 8. Generate compatibility report
    generate_compat_report(theme_dir, output, &report)?;

    Ok(report)
}

fn convert_theme_config(
    theme_dir: &Path,
    output: &Path,
    report: &mut MigrationReport,
) -> Result<()> {
    // Read theme.toml for metadata
    let theme_toml = theme_dir.join("theme.toml");
    let (name, description) = if theme_toml.exists() {
        let content = std::fs::read_to_string(&theme_toml)?;
        let val: toml::Value =
            toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()));
        (
            val.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Converted Theme")
                .to_string(),
            val.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        )
    } else {
        (
            theme_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            String::new(),
        )
    };

    // Generate mythic.toml
    let mut config = format!(
        "# Converted from Hugo theme: {name}\ntitle = \"{name}\"\nbase_url = \"http://localhost:3000\"\n"
    );
    if !description.is_empty() {
        config.push_str(&format!("# {description}\n"));
    }

    // Check for config.toml in exampleSite for additional settings
    let example_config = theme_dir.join("exampleSite/config.toml");
    if example_config.exists() {
        if let Ok(content) = std::fs::read_to_string(&example_config) {
            if let Ok(val) = toml::from_str::<toml::Value>(&content) {
                // Extract taxonomies
                if let Some(tax) = val.get("taxonomies") {
                    if let Some(table) = tax.as_table() {
                        for (key, _) in table {
                            config.push_str(&format!(
                                "\n[[taxonomies]]\nname = \"{key}\"\nslug = \"{key}\"\nfeed = true\n"
                            ));
                        }
                    }
                }
                // Extract params for highlight theme
                if let Some(params) = val.get("params") {
                    if let Some(style) = params
                        .get("highlight")
                        .or(params.get("highlightStyle"))
                        .and_then(|v| v.as_str())
                    {
                        config.push_str(&format!("\n[highlight]\ntheme = \"{style}\"\n"));
                    }
                }
            }
        }
    }

    config.push_str("\n[highlight]\ntheme = \"base16-ocean.dark\"\n");

    std::fs::write(output.join("mythic.toml"), config)?;
    report.files_converted += 1;
    Ok(())
}

fn convert_layouts(theme_dir: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let layouts_dir = theme_dir.join("layouts");
    if !layouts_dir.exists() {
        return Ok(());
    }

    let templates_dir = output.join("templates");

    for entry in WalkDir::new(&layouts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let rel = path.strip_prefix(&layouts_dir).unwrap_or(path);

        // Skip shortcodes (handled separately)
        if rel.starts_with("shortcodes") {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "html" {
            continue;
        }

        let content = std::fs::read_to_string(path)?;
        let (converted, warnings) = go_template_to_tera(&content);

        for w in &warnings {
            report.warn(format!("layouts/{}: {w}", rel.display()));
        }

        // Map Hugo layout conventions to Mythic
        // Normalize to forward slashes for consistent matching across platforms
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let target_name = map_layout_name(&rel_str);
        let target = templates_dir.join(&target_name);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, converted)?;
        report.files_converted += 1;
    }

    Ok(())
}

/// Map Hugo layout naming conventions to Mythic equivalents.
fn map_layout_name(rel_path: &str) -> String {
    rel_path
        // _default/baseof.html → base.html (Tera extends pattern)
        .replace("_default/baseof.html", "base.html")
        // _default/single.html → default.html
        .replace("_default/single.html", "default.html")
        // _default/list.html → list.html
        .replace("_default/list.html", "list.html")
        // _default/ → root level (partials/ stays as-is)
        .replace("_default/", "")
}

fn convert_assets(theme_dir: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let assets_dir = theme_dir.join("assets");
    if !assets_dir.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(&assets_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let rel = path.strip_prefix(&assets_dir).unwrap_or(path);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let target = match ext {
            "css" | "scss" | "sass" => output.join("styles").join(rel),
            "js" | "ts" => output.join("scripts").join(rel),
            _ => output.join("static").join(rel),
        };

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(path, &target)?;
        report.files_copied += 1;
    }

    Ok(())
}

fn convert_shortcodes(theme_dir: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let sc_dir = theme_dir.join("layouts/shortcodes");
    if !sc_dir.exists() {
        return Ok(());
    }

    let out_sc = output.join("shortcodes");
    std::fs::create_dir_all(&out_sc)?;

    for entry in std::fs::read_dir(&sc_dir)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let (mut converted, warnings) = go_template_to_tera(&content);

        // Hugo shortcode-specific conversions
        converted = converted
            .replace("{{ .Get \"", "{{ ")
            .replace("\" }}", " }}")
            .replace("{{ .Get 0 }}", "{{ _arg0 }}")
            .replace("{{ .Inner }}", "{{ inner | safe }}")
            .replace("{{.Inner}}", "{{ inner | safe }}")
            .replace("{{ .Page.Title }}", "{{ page.title }}")
            .replace("{{ .Page.Permalink }}", "{{ page.url }}");

        for w in warnings {
            report.warn(format!(
                "shortcodes/{}: {w}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }

        if let Some(fname) = path.file_name() {
            std::fs::write(out_sc.join(fname), converted)?;
            report.files_converted += 1;
        }
    }

    Ok(())
}

fn convert_archetypes(theme_dir: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let archetypes = theme_dir.join("archetypes");
    if !archetypes.exists() {
        return Ok(());
    }

    // Convert archetypes to example content files
    let content_dir = output.join("content");
    std::fs::create_dir_all(&content_dir)?;

    for entry in std::fs::read_dir(&archetypes)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;

        // Hugo archetypes use Go template syntax in frontmatter
        // Replace common patterns
        let converted = content
            .replace("{{ .Name }}", "Example Post")
            .replace("{{ .Date }}", "2024-01-01")
            .replace("{{ .Type }}", "post");

        if let Some(fname) = path.file_name() {
            let target_name = fname.to_string_lossy().replace("default", "example");
            std::fs::write(content_dir.join(target_name.as_str()), converted)?;
            report.files_converted += 1;
        }
    }

    Ok(())
}

fn convert_example_site(
    theme_dir: &Path,
    output: &Path,
    report: &mut MigrationReport,
) -> Result<()> {
    let example_site = theme_dir.join("exampleSite");
    if !example_site.exists() {
        return Ok(());
    }

    // Copy example content
    let example_content = example_site.join("content");
    if example_content.exists() {
        copy_dir(&example_content, &output.join("content"), report)?;
    }

    // Copy example data
    let example_data = example_site.join("data");
    if example_data.exists() {
        copy_dir(&example_data, &output.join("_data"), report)?;
    }

    // Copy example static
    let example_static = example_site.join("static");
    if example_static.exists() {
        copy_dir(&example_static, &output.join("static"), report)?;
    }

    Ok(())
}

fn generate_compat_report(theme_dir: &Path, output: &Path, report: &MigrationReport) -> Result<()> {
    let mut md = String::from("# Theme Conversion Report\n\n");
    md.push_str(&format!("Converted from: `{}`\n\n", theme_dir.display()));
    md.push_str(&format!("Files converted: {}\n", report.files_converted));
    md.push_str(&format!("Files copied: {}\n\n", report.files_copied));

    if !report.warnings.is_empty() {
        md.push_str("## Warnings (may need manual attention)\n\n");
        for w in &report.warnings {
            md.push_str(&format!("- {w}\n"));
        }
        md.push('\n');
    }

    md.push_str("## Next Steps\n\n");
    md.push_str("1. Run `mythic build` to check for template errors\n");
    md.push_str("2. Fix any Tera template syntax issues in `templates/`\n");
    md.push_str(
        "3. Replace Hugo Pipes (`resources.Get | toCSS | minify`) with Mythic's asset pipeline\n",
    );
    md.push_str("4. Replace `{{ with }}` blocks with `{% if %}` equivalents\n");
    md.push_str("5. Test with `mythic serve`\n");

    std::fs::write(output.join("CONVERSION.md"), md)?;
    Ok(())
}

fn copy_dir(src: &Path, dest: &Path, report: &mut MigrationReport) -> Result<()> {
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
    fn converts_basic_theme() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        // Create minimal Hugo theme structure
        std::fs::create_dir_all(src.path().join("layouts/_default")).unwrap();
        std::fs::write(
            src.path().join("theme.toml"),
            "name = \"Test Theme\"\ndescription = \"A test theme\"",
        )
        .unwrap();
        std::fs::write(
            src.path().join("layouts/_default/baseof.html"),
            "<!DOCTYPE html><html><head><title>{{ .Title }}</title></head><body>{{ block \"main\" . }}{{ end }}</body></html>",
        ).unwrap();
        std::fs::write(
            src.path().join("layouts/_default/single.html"),
            "{{ define \"main\" }}<article>{{ .Content }}</article>{{ end }}",
        )
        .unwrap();

        let report = convert_theme(src.path(), out.path()).unwrap();
        assert!(report.files_converted >= 3); // config + 2 templates

        // Check mythic.toml created
        let config = std::fs::read_to_string(out.path().join("mythic.toml")).unwrap();
        assert!(config.contains("Test Theme"));

        // Check templates converted
        assert!(out.path().join("templates/base.html").exists());
        assert!(out.path().join("templates/default.html").exists());

        // Check Tera syntax in converted templates
        let base = std::fs::read_to_string(out.path().join("templates/base.html")).unwrap();
        assert!(base.contains("{{ page.title }}"));
        assert!(base.contains("{% block main %}"));

        // Check conversion report
        assert!(out.path().join("CONVERSION.md").exists());
    }

    #[test]
    fn converts_shortcodes() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(src.path().join("layouts/shortcodes")).unwrap();
        std::fs::write(
            src.path().join("layouts/shortcodes/youtube.html"),
            "<iframe src=\"https://youtube.com/embed/{{ .Get \"id\" }}\"></iframe>",
        )
        .unwrap();

        convert_theme(src.path(), out.path()).unwrap();

        let sc = std::fs::read_to_string(out.path().join("shortcodes/youtube.html")).unwrap();
        assert!(sc.contains("{{ id"));
        assert!(!sc.contains(".Get"));
    }

    #[test]
    fn handles_assets_directory() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(src.path().join("assets/css")).unwrap();
        std::fs::create_dir_all(src.path().join("assets/js")).unwrap();
        std::fs::write(src.path().join("assets/css/main.css"), "body {}").unwrap();
        std::fs::write(src.path().join("assets/js/app.js"), "// js").unwrap();

        convert_theme(src.path(), out.path()).unwrap();

        assert!(out.path().join("styles/css/main.css").exists());
        assert!(out.path().join("scripts/js/app.js").exists());
    }

    #[test]
    fn layout_name_mapping() {
        assert_eq!(map_layout_name("_default/baseof.html"), "base.html");
        assert_eq!(map_layout_name("_default/single.html"), "default.html");
        assert_eq!(map_layout_name("_default/list.html"), "list.html");
        assert_eq!(
            map_layout_name("partials/header.html"),
            "partials/header.html"
        );
    }

    #[test]
    fn converts_example_site() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(src.path().join("exampleSite/content/posts")).unwrap();
        std::fs::write(
            src.path().join("exampleSite/content/posts/hello.md"),
            "---\ntitle: Hello\n---\n# Hello World",
        )
        .unwrap();

        convert_theme(src.path(), out.path()).unwrap();

        assert!(out.path().join("content/posts/hello.md").exists());
    }
}
