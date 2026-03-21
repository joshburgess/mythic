//! Directory-level data cascade (Eleventy-style _dir.yaml).

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::page::Page;

/// Apply the data cascade to all pages.
///
/// Merge order (lowest to highest priority):
/// root `_dir.yaml` → nested `_dir.yaml` → page frontmatter
pub fn apply_cascade(pages: &mut [Page], content_dir: &Path) -> Result<()> {
    let dir_data = collect_dir_data(content_dir)?;

    for page in pages.iter_mut() {
        let rel_dir = page
            .source_path
            .parent()
            .and_then(|p| p.strip_prefix(content_dir).ok())
            .unwrap_or_else(|| Path::new(""));

        // Gather cascade: from root down to this page's directory
        let mut merged = Value::Object(serde_json::Map::new());
        let mut current = PathBuf::new();

        // Root _dir data
        if let Some(data) = dir_data.get(Path::new("")) {
            deep_merge(&mut merged, data);
        }

        // Walk down the path components
        for component in rel_dir.components() {
            current = current.join(component);
            if let Some(data) = dir_data.get(current.as_path()) {
                deep_merge(&mut merged, data);
            }
        }

        // Apply cascaded data to page frontmatter
        if let Value::Object(map) = merged {
            for (key, value) in map {
                apply_to_frontmatter(&mut page.frontmatter, &key, value);
            }
        }
    }

    Ok(())
}

/// Collect all _dir.{yaml,toml,json} files, keyed by their relative directory.
fn collect_dir_data(content_dir: &Path) -> Result<HashMap<PathBuf, Value>> {
    let mut result = HashMap::new();

    for entry in walkdir::WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let is_dir_data = matches!(
            name,
            "_dir.yaml" | "_dir.yml" | "_dir.toml" | "_dir.json"
        );
        if !is_dir_data {
            continue;
        }

        let rel_dir = path
            .parent()
            .and_then(|p| p.strip_prefix(content_dir).ok())
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;

        let value: Value = match ext {
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .with_context(|| format!("Invalid YAML: {}", path.display()))?,
            "toml" => {
                let tv: toml::Value = toml::from_str(&content)
                    .with_context(|| format!("Invalid TOML: {}", path.display()))?;
                crate::data::toml_to_json_pub(tv)
            }
            "json" => serde_json::from_str(&content)
                .with_context(|| format!("Invalid JSON: {}", path.display()))?,
            _ => continue,
        };

        result.insert(rel_dir, value);
    }

    Ok(result)
}

/// Deep merge `source` into `target`. Maps are merged recursively; other values are overwritten.
pub fn deep_merge(target: &mut Value, source: &Value) {
    match (target, source) {
        (Value::Object(t), Value::Object(s)) => {
            for (key, val) in s {
                let entry = t.entry(key.clone()).or_insert(Value::Null);
                deep_merge(entry, val);
            }
        }
        (target, source) => {
            *target = source.clone();
        }
    }
}

fn apply_to_frontmatter(fm: &mut crate::page::Frontmatter, key: &str, value: Value) {
    // Only apply if the frontmatter field hasn't been explicitly set
    match key {
        "layout" => {
            if fm.layout.as_deref() == Some("default") || fm.layout.is_none() {
                if let Value::String(s) = value {
                    fm.layout = Some(s);
                }
            }
        }
        "draft" => {
            if fm.draft.is_none() {
                if let Value::Bool(b) = value {
                    fm.draft = Some(b);
                }
            }
        }
        "tags" => {
            if fm.tags.is_none() {
                if let Value::Array(arr) = value {
                    let tags: Vec<String> = arr
                        .into_iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    if !tags.is_empty() {
                        fm.tags = Some(tags);
                    }
                }
            }
        }
        _ => {
            // Store in extra
            let extra = fm.extra.get_or_insert_with(HashMap::new);
            extra.entry(key.to_string()).or_insert(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::collections::HashMap;

    fn make_page(source: &Path, slug: &str) -> Page {
        Page {
            source_path: source.to_path_buf(),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: "Test".to_string(),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn basic_cascade() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("_dir.yaml"), "layout: blog\nauthor: Alice").unwrap();
        std::fs::write(content.join("post.md"), "---\ntitle: Post\n---\nContent").unwrap();

        let mut pages = vec![make_page(&content.join("post.md"), "post")];
        apply_cascade(&mut pages, &content).unwrap();

        assert_eq!(pages[0].frontmatter.layout.as_deref(), Some("blog"));
        assert_eq!(
            pages[0].frontmatter.extra.as_ref().unwrap()["author"],
            Value::String("Alice".to_string())
        );
    }

    #[test]
    fn nested_override() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        let blog = content.join("blog");
        std::fs::create_dir_all(&blog).unwrap();
        std::fs::write(content.join("_dir.yaml"), "layout: base\nauthor: Root").unwrap();
        std::fs::write(blog.join("_dir.yaml"), "layout: post\nauthor: Blog").unwrap();
        std::fs::write(blog.join("entry.md"), "---\ntitle: Entry\n---\nBody").unwrap();

        let mut pages = vec![make_page(&blog.join("entry.md"), "blog/entry")];
        apply_cascade(&mut pages, &content).unwrap();

        assert_eq!(pages[0].frontmatter.layout.as_deref(), Some("post"));
        assert_eq!(
            pages[0].frontmatter.extra.as_ref().unwrap()["author"],
            Value::String("Blog".to_string())
        );
    }

    #[test]
    fn page_frontmatter_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("_dir.yaml"), "draft: true").unwrap();

        let mut pages = vec![make_page(&content.join("page.md"), "page")];
        pages[0].frontmatter.draft = Some(false);
        apply_cascade(&mut pages, &content).unwrap();

        // Page-level draft=false should win over cascade draft=true
        assert_eq!(pages[0].frontmatter.draft, Some(false));
    }

    #[test]
    fn deep_merge_behavior() {
        let mut a = serde_json::json!({"theme": {"color": "red", "size": 12}});
        let b = serde_json::json!({"theme": {"color": "blue", "font": "sans"}});
        deep_merge(&mut a, &b);

        assert_eq!(a["theme"]["color"], "blue");
        assert_eq!(a["theme"]["size"], 12);
        assert_eq!(a["theme"]["font"], "sans");
    }
}
