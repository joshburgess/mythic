//! Jekyll to Mythic migration.

use anyhow::{Context, Result};
use std::path::Path;
use walkdir::WalkDir;

use super::convert::liquid_to_tera;
use super::MigrationReport;

/// Migrate a Jekyll site to Mythic format.
pub fn migrate(source: &Path, output: &Path) -> Result<MigrationReport> {
    let mut report = MigrationReport::default();

    std::fs::create_dir_all(output)?;

    // 1. Convert config
    convert_config(source, output, &mut report)?;

    // 2. Migrate posts
    migrate_posts(source, output, &mut report)?;

    // 3. Convert layouts
    migrate_layouts(source, output, &mut report)?;

    // 4. Copy includes
    copy_includes(source, output, &mut report)?;

    // 5. Copy data
    copy_dir_if_exists(&source.join("_data"), &output.join("_data"), &mut report)?;

    // 6. Copy static assets
    copy_static_assets(source, output, &mut report)?;

    Ok(report)
}

fn convert_config(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let config_path = source.join("_config.yml");
    if !config_path.exists() {
        report.warn("No _config.yml found");
        // Write a minimal config
        std::fs::write(
            output.join("mythic.toml"),
            "title = \"Migrated Site\"\nbase_url = \"http://localhost:3000\"\n",
        )?;
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path)?;
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).context("Failed to parse _config.yml")?;

    let title = yaml["title"].as_str().unwrap_or("Migrated Site");
    let base_url = yaml["url"]
        .as_str()
        .or(yaml["baseurl"].as_str())
        .unwrap_or("http://localhost:3000");
    let description = yaml["description"].as_str();

    let mut toml = format!("title = \"{title}\"\nbase_url = \"{base_url}\"\n");
    if let Some(desc) = description {
        toml.push_str(&format!("description = \"{desc}\"\n"));
    }

    std::fs::write(output.join("mythic.toml"), toml)?;
    report.files_converted += 1;

    Ok(())
}

fn migrate_posts(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let posts_dir = source.join("_posts");
    if !posts_dir.exists() {
        return Ok(());
    }

    let out_posts = output.join("content/posts");
    std::fs::create_dir_all(&out_posts)?;

    for entry in WalkDir::new(&posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let filename = path.file_name().unwrap_or_default().to_string_lossy();

        // Jekyll format: YYYY-MM-DD-title.md
        let content = std::fs::read_to_string(path)?;

        if let Some((date, slug)) = parse_jekyll_filename(&filename) {
            let new_content = inject_date_if_missing(&content, &date);
            std::fs::write(out_posts.join(format!("{slug}.md")), new_content)?;
        } else {
            std::fs::write(out_posts.join(filename.as_ref()), &content)?;
        }

        report.files_converted += 1;
    }

    Ok(())
}

fn parse_jekyll_filename(filename: &str) -> Option<(String, String)> {
    // Expected: YYYY-MM-DD-title.ext
    let stem = filename
        .strip_suffix(".md")
        .or_else(|| filename.strip_suffix(".markdown"))?;

    if stem.len() < 11 {
        return None;
    }

    let date_part = &stem[..10];
    // Validate date format
    if date_part.chars().nth(4) != Some('-') || date_part.chars().nth(7) != Some('-') {
        return None;
    }

    let slug = &stem[11..];
    if slug.is_empty() {
        return None;
    }

    Some((date_part.to_string(), slug.to_string()))
}

fn inject_date_if_missing(content: &str, date: &str) -> String {
    if let Some(after_open) = content.strip_prefix("---") {
        if let Some(end) = after_open.find("\n---") {
            let frontmatter = &after_open[..end];
            if !frontmatter.contains("date:") {
                let new_fm = format!("{frontmatter}\ndate: \"{date}\"");
                return format!("---\n{new_fm}\n---{}", &after_open[end + 4..]);
            }
        }
    }
    content.to_string()
}

fn migrate_layouts(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let layouts_dir = source.join("_layouts");
    if !layouts_dir.exists() {
        return Ok(());
    }

    let out_templates = output.join("templates");
    std::fs::create_dir_all(&out_templates)?;

    for entry in WalkDir::new(&layouts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let rel = path.strip_prefix(&layouts_dir).unwrap_or(path);
        let content = std::fs::read_to_string(path)?;
        let (converted, warnings) = liquid_to_tera(&content);

        for w in warnings {
            report.warn(format!("{}: {w}", rel.display()));
        }

        std::fs::write(out_templates.join(rel), converted)?;
        report.files_converted += 1;
    }

    Ok(())
}

fn copy_includes(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    let includes_dir = source.join("_includes");
    if !includes_dir.exists() {
        return Ok(());
    }

    let out_partials = output.join("templates/partials");
    copy_dir_if_exists(&includes_dir, &out_partials, report)?;

    Ok(())
}

fn copy_static_assets(source: &Path, output: &Path, report: &mut MigrationReport) -> Result<()> {
    // Jekyll serves files not starting with _ from root
    for entry in std::fs::read_dir(source)?.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip Jekyll special dirs and files
        if name_str.starts_with('_') || name_str.starts_with('.') {
            continue;
        }
        if name_str == "Gemfile" || name_str == "Gemfile.lock" {
            continue;
        }

        let src = entry.path();
        let dest = output.join("static").join(&*name_str);

        if src.is_dir() {
            copy_dir_if_exists(&src, &dest, report)?;
        } else if src.is_file() {
            let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(
                ext,
                "css"
                    | "js"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "svg"
                    | "ico"
                    | "webp"
                    | "woff"
                    | "woff2"
                    | "ttf"
                    | "eot"
            ) {
                std::fs::create_dir_all(dest.parent().unwrap())?;
                std::fs::copy(&src, &dest)?;
                report.files_copied += 1;
            }
        }
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
    fn config_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(
            src.path().join("_config.yml"),
            "title: My Blog\nurl: https://example.com\ndescription: A test blog",
        )
        .unwrap();

        let report = migrate(src.path(), out.path()).unwrap();
        assert!(report.files_converted > 0);

        let config = std::fs::read_to_string(out.path().join("mythic.toml")).unwrap();
        assert!(config.contains("My Blog"));
        assert!(config.contains("https://example.com"));
    }

    #[test]
    fn post_migration() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join("_config.yml"), "title: Test").unwrap();

        let posts = src.path().join("_posts");
        std::fs::create_dir_all(&posts).unwrap();
        std::fs::write(
            posts.join("2024-01-15-my-first-post.md"),
            "---\ntitle: My First Post\n---\n# Hello\n\nContent here.",
        )
        .unwrap();

        let report = migrate(src.path(), out.path()).unwrap();
        assert!(report.files_converted >= 2); // config + post

        let post = out.path().join("content/posts/my-first-post.md");
        assert!(post.exists());
        let content = std::fs::read_to_string(post).unwrap();
        assert!(content.contains("date: \"2024-01-15\""));
    }

    #[test]
    fn template_conversion() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();

        std::fs::write(src.path().join("_config.yml"), "title: Test").unwrap();

        let layouts = src.path().join("_layouts");
        std::fs::create_dir_all(&layouts).unwrap();
        std::fs::write(
            layouts.join("default.html"),
            "<html><body>{{ content }}</body></html>",
        )
        .unwrap();

        migrate(src.path(), out.path()).unwrap();

        let template = std::fs::read_to_string(out.path().join("templates/default.html")).unwrap();
        assert!(template.contains("{{ content | safe }}"));
    }

    #[test]
    fn parse_jekyll_filename_works() {
        let (date, slug) = parse_jekyll_filename("2024-01-15-hello-world.md").unwrap();
        assert_eq!(date, "2024-01-15");
        assert_eq!(slug, "hello-world");
    }

    #[test]
    fn parse_jekyll_filename_invalid() {
        assert!(parse_jekyll_filename("not-a-date.md").is_none());
    }
}
