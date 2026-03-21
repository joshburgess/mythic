//! Data file loading from the _data/ directory.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

/// Load all data files (YAML, TOML, JSON) from the data directory.
///
/// Returns a nested JSON Value where file paths map to namespaces:
/// `_data/authors.yaml` → `data.authors`
/// `_data/nav/main.yaml` → `data.nav.main`
pub fn load_data(data_dir: &Path) -> Result<Value> {
    let mut root = serde_json::Map::new();

    if !data_dir.exists() {
        return Ok(Value::Object(root));
    }

    for entry in WalkDir::new(data_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip files starting with _ (like _dir.yaml used by cascade)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('_') {
                continue;
            }
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let value = match ext {
            "yaml" | "yml" => {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read data file: {}", path.display()))?;
                serde_yaml::from_str::<Value>(&content)
                    .with_context(|| format!("Invalid YAML in data file: {}", path.display()))?
            }
            "toml" => {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read data file: {}", path.display()))?;
                let toml_val: toml::Value = toml::from_str(&content)
                    .with_context(|| format!("Invalid TOML in data file: {}", path.display()))?;
                toml_to_json(toml_val)
            }
            "json" => {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read data file: {}", path.display()))?;
                serde_json::from_str::<Value>(&content)
                    .with_context(|| format!("Invalid JSON in data file: {}", path.display()))?
            }
            _ => continue,
        };

        // Build namespace path from relative directory + file stem
        let rel = path.strip_prefix(data_dir).unwrap_or(path);
        let stem = rel.with_extension("");
        let parts: Vec<&str> = stem
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        insert_nested(&mut root, &parts, value);
    }

    Ok(Value::Object(root))
}

fn insert_nested(map: &mut serde_json::Map<String, Value>, parts: &[&str], value: Value) {
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        map.insert(parts[0].to_string(), value);
        return;
    }
    let entry = map
        .entry(parts[0].to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if let Value::Object(ref mut inner) = entry {
        insert_nested(inner, &parts[1..], value);
    }
}

/// Convert a TOML value to a JSON value.
pub fn toml_to_json_pub(val: toml::Value) -> Value {
    toml_to_json(val)
}

fn toml_to_json(val: toml::Value) -> Value {
    match val {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(serde_json::Number::from(i)),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let map: serde_json::Map<String, Value> = t
                .into_iter()
                .map(|(k, v)| (k, toml_to_json(v)))
                .collect();
            Value::Object(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_yaml_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("authors.yaml"),
            "- name: Alice\n  role: admin\n- name: Bob\n  role: editor",
        )
        .unwrap();

        let data = load_data(dir.path()).unwrap();
        let authors = &data["authors"];
        assert!(authors.is_array());
        assert_eq!(authors[0]["name"], "Alice");
        assert_eq!(authors[1]["role"], "editor");
    }

    #[test]
    fn load_toml_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("site.toml"),
            "[social]\ntwitter = \"@example\"\ngithub = \"example\"",
        )
        .unwrap();

        let data = load_data(dir.path()).unwrap();
        assert_eq!(data["site"]["social"]["twitter"], "@example");
    }

    #[test]
    fn load_json_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("menu.json"),
            r#"[{"label": "Home", "url": "/"}, {"label": "About", "url": "/about"}]"#,
        )
        .unwrap();

        let data = load_data(dir.path()).unwrap();
        assert_eq!(data["menu"][0]["label"], "Home");
    }

    #[test]
    fn nested_directory_namespace() {
        let dir = tempfile::tempdir().unwrap();
        let nav = dir.path().join("nav");
        std::fs::create_dir_all(&nav).unwrap();
        std::fs::write(
            nav.join("main.yaml"),
            "- label: Home\n  url: /",
        )
        .unwrap();

        let data = load_data(dir.path()).unwrap();
        assert_eq!(data["nav"]["main"][0]["label"], "Home");
    }

    #[test]
    fn invalid_file_produces_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.yaml"), ": : : not valid").unwrap();

        let result = load_data(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("bad.yaml"));
    }

    #[test]
    fn nonexistent_dir_returns_empty() {
        let data = load_data(Path::new("/nonexistent/_data")).unwrap();
        assert_eq!(data, Value::Object(serde_json::Map::new()));
    }

    #[test]
    fn mixed_file_types_in_same_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.yaml"), "key: from_yaml").unwrap();
        std::fs::write(dir.path().join("settings.json"), r#"{"key": "from_json"}"#).unwrap();
        std::fs::write(dir.path().join("meta.toml"), "key = \"from_toml\"").unwrap();

        let data = load_data(dir.path()).unwrap();
        assert_eq!(data["config"]["key"], "from_yaml");
        assert_eq!(data["settings"]["key"], "from_json");
        assert_eq!(data["meta"]["key"], "from_toml");
    }

    #[test]
    fn deeply_nested_data_directories() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("leaf.yaml"), "value: deep").unwrap();

        let data = load_data(dir.path()).unwrap();
        assert_eq!(data["a"]["b"]["c"]["leaf"]["value"], "deep");
    }

    #[test]
    fn empty_data_file() {
        let dir = tempfile::tempdir().unwrap();
        // An empty YAML file parses as Null
        std::fs::write(dir.path().join("empty.yaml"), "").unwrap();

        let data = load_data(dir.path()).unwrap();
        assert!(data["empty"].is_null());
    }

    #[test]
    fn data_file_with_root_array() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("items.json"),
            r#"[1, 2, 3, "four"]"#,
        )
        .unwrap();

        let data = load_data(dir.path()).unwrap();
        assert!(data["items"].is_array());
        assert_eq!(data["items"][0], 1);
        assert_eq!(data["items"][3], "four");
    }

    #[test]
    fn data_file_with_complex_nested_structures() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("complex.json"),
            r#"{
                "users": [
                    {"name": "Alice", "roles": ["admin", "editor"]},
                    {"name": "Bob", "roles": ["viewer"]}
                ],
                "settings": {
                    "nested": {"deep": {"value": 42}}
                }
            }"#,
        )
        .unwrap();

        let data = load_data(dir.path()).unwrap();
        assert_eq!(data["complex"]["users"][0]["name"], "Alice");
        assert_eq!(data["complex"]["users"][0]["roles"][1], "editor");
        assert_eq!(data["complex"]["settings"]["nested"]["deep"]["value"], 42);
    }

    #[test]
    fn underscore_prefixed_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("_dir.yaml"), "layout: blog").unwrap();
        std::fs::write(dir.path().join("_hidden.json"), r#"{"secret": true}"#).unwrap();
        std::fs::write(dir.path().join("visible.yaml"), "ok: true").unwrap();

        let data = load_data(dir.path()).unwrap();
        assert!(data.get("_dir").is_none());
        assert!(data.get("_hidden").is_none());
        assert_eq!(data["visible"]["ok"], true);
    }
}
