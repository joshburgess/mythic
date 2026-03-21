//! CSS concatenation and minification.

use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Concatenate all `.css` files in the directory (sorted alphabetically).
pub fn concat_css(styles_dir: &Path) -> Result<String> {
    let mut entries: Vec<_> = std::fs::read_dir(styles_dir)
        .with_context(|| format!("Failed to read styles dir: {}", styles_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x == "css")
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by_key(|e| e.file_name());

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

/// Basic CSS minification: strip comments, collapse whitespace.
pub fn minify_css(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    let mut in_string = false;
    let mut string_char = '"';

    while let Some(c) = chars.next() {
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

        if c == '"' || c == '\'' {
            in_string = true;
            string_char = c;
            out.push(c);
            continue;
        }

        // Strip block comments
        if c == '/' {
            if chars.peek() == Some(&'*') {
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
        }

        // Collapse whitespace
        if c.is_whitespace() {
            // Skip consecutive whitespace
            while chars.peek().map(|ch| ch.is_whitespace()).unwrap_or(false) {
                chars.next();
            }
            // Only emit a space if not adjacent to a special char
            let last = out.chars().last();
            let next = chars.peek().copied();
            let skip = matches!(last, Some('{' | '}' | ';' | ':' | ',' | '>' | '+' | '~'))
                || matches!(next, Some('{' | '}' | ';' | ':' | ',' | '>' | '+' | '~'));
            if !skip && !out.is_empty() && next.is_some() {
                out.push(' ');
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// Write CSS to a content-hashed file, return the relative path.
pub fn write_hashed(css: &str, output_dir: &Path) -> Result<String> {
    let hash = {
        let mut h = DefaultHasher::new();
        css.hash(&mut h);
        format!("{:x}", h.finish())
    };

    let filename = format!("styles-{hash}.css");
    let dest = output_dir.join(&filename);
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(&dest, css)
        .with_context(|| format!("Failed to write: {}", dest.display()))?;

    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_css_alphabetical() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.css"), "b { color: blue; }").unwrap();
        std::fs::write(dir.path().join("a.css"), "a { color: red; }").unwrap();

        let result = concat_css(dir.path()).unwrap();
        let a_pos = result.find("color: red").unwrap();
        let b_pos = result.find("color: blue").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn minify_strips_comments_and_whitespace() {
        let css = r#"
/* Main styles */
body {
    margin: 0;
    padding: 0;
}

/* Header */
h1 {
    color: red;
}
"#;
        let minified = minify_css(css);
        assert!(!minified.contains("/*"));
        assert!(!minified.contains("Main styles"));
        assert!(minified.contains("margin:0"));
        assert!(minified.contains("body{"));
    }

    #[test]
    fn hash_changes_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let path1 = write_hashed("body { color: red; }", dir.path()).unwrap();
        let path2 = write_hashed("body { color: blue; }", dir.path()).unwrap();
        assert_ne!(path1, path2);
    }

    #[test]
    fn minify_preserves_strings() {
        let css = r#"body::after { content: "  hello  world  "; }"#;
        let minified = minify_css(css);
        assert!(minified.contains("\"  hello  world  \""));
    }
}
