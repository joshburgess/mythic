//! JavaScript concatenation and minification.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Concatenate all `.js` files in the directory tree (sorted by path for deterministic output).
pub fn concat_js(scripts_dir: &Path) -> Result<String> {
    let mut entries: Vec<_> = walkdir::WalkDir::new(scripts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x == "js")
                    .unwrap_or(false)
        })
        .collect();

    entries.sort_by(|a, b| a.path().cmp(b.path()));

    let mut combined = String::new();
    for entry in entries {
        let content = std::fs::read_to_string(entry.path())
            .with_context(|| format!("Failed to read: {}", entry.path().display()))?;
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&content);
    }

    Ok(combined)
}

/// Basic JS minification: strip single-line comments, collapse whitespace.
/// Preserves string literals and template literals.
///
/// **Limitation:** Regex literals (`/pattern/flags`) are not recognized. A regex
/// literal that starts with `//` will be mis-parsed as a single-line comment,
/// and one starting with `/*` as a block comment. If your JavaScript relies on
/// regex literals, pre-minify the file with a full-featured tool (e.g. esbuild,
/// terser) before passing it to Mythic.
pub fn minify_js(js: &str) -> String {
    let mut out = String::with_capacity(js.len());
    let mut chars = js.chars().peekable();
    let mut in_string = false;
    let mut string_char = '"';
    let mut in_template = false;

    while let Some(c) = chars.next() {
        // Handle template literals
        if in_template {
            out.push(c);
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    out.push(next);
                    chars.next();
                }
            } else if c == '`' {
                in_template = false;
            }
            continue;
        }

        // Handle string literals
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    out.push(next);
                    chars.next();
                }
            } else if c == string_char {
                in_string = false;
            }
            continue;
        }

        if c == '`' {
            in_template = true;
            out.push(c);
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = true;
            string_char = c;
            out.push(c);
            continue;
        }

        // Strip single-line comments
        if c == '/' && chars.peek() == Some(&'/') {
            while let Some(&next) = chars.peek() {
                if next == '\n' {
                    break;
                }
                chars.next();
            }
            continue;
        }

        // Strip block comments
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            loop {
                match chars.next() {
                    Some('*') if chars.peek() == Some(&'/') => {
                        chars.next();
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            continue;
        }

        // Collapse whitespace (preserve newlines as single newline for ASI)
        if c.is_whitespace() {
            let mut has_newline = c == '\n';
            while chars.peek().map(|ch| ch.is_whitespace()).unwrap_or(false) {
                if chars.peek() == Some(&'\n') {
                    has_newline = true;
                }
                chars.next();
            }
            if has_newline {
                out.push('\n');
            } else if !out.is_empty() && chars.peek().is_some() {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// Write JS to a content-hashed file, return the relative path.
pub fn write_hashed(js: &str, output_dir: &Path) -> Result<String> {
    let digest = Sha256::digest(js.as_bytes());
    let hash = &format!("{:x}", digest)[..16];

    let filename = format!("scripts-{hash}.js");
    let dest = output_dir.join(&filename);
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(&dest, js).with_context(|| format!("Failed to write: {}", dest.display()))?;

    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_js_alphabetical() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.js"), "var b = 2;").unwrap();
        std::fs::write(dir.path().join("a.js"), "var a = 1;").unwrap();

        let result = concat_js(dir.path()).unwrap();
        let a_pos = result.find("var a").unwrap();
        let b_pos = result.find("var b").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn minify_strips_comments() {
        let js = r#"
// This is a comment
var x = 1;
/* block comment */
var y = 2;
"#;
        let minified = minify_js(js);
        assert!(!minified.contains("This is a comment"));
        assert!(!minified.contains("block comment"));
        assert!(minified.contains("var x = 1;"));
        assert!(minified.contains("var y = 2;"));
    }

    #[test]
    fn hash_changes_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let path1 = write_hashed("var x = 1;", dir.path()).unwrap();
        let path2 = write_hashed("var x = 2;", dir.path()).unwrap();
        assert_ne!(path1, path2);
    }

    #[test]
    fn minify_preserves_strings() {
        let js = r#"var s = "  hello  world  ";"#;
        let minified = minify_js(js);
        assert!(minified.contains("\"  hello  world  \""));
    }

    #[test]
    fn minify_preserves_template_literals() {
        let js = "var s = `  hello\n  world  `;";
        let minified = minify_js(js);
        assert!(minified.contains("`  hello\n  world  `"));
    }
}
