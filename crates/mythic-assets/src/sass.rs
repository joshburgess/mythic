//! Sass/SCSS compilation using the grass crate.

use anyhow::{Context, Result};
use std::path::Path;

/// Compile all Sass/SCSS files and concatenate with plain CSS files.
///
/// Order: alphabetical, with `.scss`/`.sass` files compiled to CSS first.
pub fn compile_and_concat(styles_dir: &Path) -> Result<String> {
    let mut entries: Vec<_> = walkdir::WalkDir::new(styles_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            if !e.file_type().is_file() {
                return false;
            }
            let name = e.file_name().to_string_lossy().to_string();
            // Skip Sass partials (files starting with '_') — they are meant
            // to be @imported, not compiled independently.
            if name.starts_with('_') {
                return false;
            }
            matches!(
                e.path().extension().and_then(|x| x.to_str()),
                Some("css" | "scss" | "sass")
            )
        })
        .collect();

    entries.sort_by(|a, b| a.path().cmp(b.path()));

    let mut combined = String::new();
    for entry in entries {
        let path = entry.path().to_path_buf();
        let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");

        let css = match ext {
            "scss" | "sass" => compile_file(&path)?,
            _ => std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read: {}", path.display()))?,
        };

        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&css);
    }

    Ok(combined)
}

/// Compile a single Sass/SCSS file to CSS.
pub fn compile_file(path: &Path) -> Result<String> {
    let options = grass::Options::default().load_path(path.parent().unwrap_or(Path::new(".")));

    let css = grass::from_path(path, &options)
        .map_err(|e| anyhow::anyhow!("Sass compilation error in {}: {}", path.display(), e))?;

    Ok(css)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_scss_compilation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("main.scss"),
            "$color: red;\nbody { color: $color; }",
        )
        .unwrap();

        let result = compile_and_concat(dir.path()).unwrap();
        assert!(result.contains("color: red"));
        assert!(!result.contains("$color"));
    }

    #[test]
    fn scss_imports() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("_variables.scss"), "$primary: blue;").unwrap();
        std::fs::write(
            dir.path().join("main.scss"),
            "@import 'variables';\nbody { color: $primary; }",
        )
        .unwrap();

        let result = compile_and_concat(dir.path()).unwrap();
        assert!(result.contains("color: blue"));
    }

    #[test]
    fn mixed_css_and_scss() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.css"), "a { color: green; }").unwrap();
        std::fs::write(
            dir.path().join("b.scss"),
            "$size: 16px;\np { font-size: $size; }",
        )
        .unwrap();

        let result = compile_and_concat(dir.path()).unwrap();
        assert!(result.contains("color: green"));
        assert!(result.contains("font-size: 16px"));
        assert!(!result.contains("$size"));
    }
}
