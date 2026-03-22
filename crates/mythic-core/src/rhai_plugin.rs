//! Rhai scripting engine integration for user-defined plugins.

use anyhow::{Context, Result};
use rhai::{Dynamic, Engine, Scope, AST};
use std::path::Path;

use crate::page::Page;
use crate::plugin::Plugin;

/// A plugin loaded from a Rhai script file.
pub struct RhaiPlugin {
    script_name: String,
    ast: AST,
    engine: Engine,
}

impl RhaiPlugin {
    /// Load a Rhai plugin from a script file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let engine = Engine::new();
        let ast = engine.compile_file(path.into()).map_err(|e| {
            anyhow::anyhow!(
                "Failed to compile Rhai script {}: {}",
                path.display(),
                e
            )
        })?;

        let script_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(RhaiPlugin {
            script_name,
            ast,
            engine,
        })
    }
}

impl Plugin for RhaiPlugin {
    fn name(&self) -> &str {
        &self.script_name
    }

    fn on_page_discovered(&self, page: &mut Page) -> Result<()> {
        run_hook(&self.engine, &self.ast, "on_page_discovered", page, &self.script_name)
    }

    fn on_pre_render(&self, page: &mut Page) -> Result<()> {
        run_hook(&self.engine, &self.ast, "on_pre_render", page, &self.script_name)
    }

    fn on_post_render(&self, page: &mut Page) -> Result<()> {
        run_hook(&self.engine, &self.ast, "on_post_render", page, &self.script_name)
    }
}

fn run_hook(engine: &Engine, ast: &AST, hook_name: &str, page: &mut Page, script_name: &str) -> Result<()> {
    // Check if the function exists in the script
    let has_fn = ast.iter_functions().any(|f| f.name == hook_name);
    if !has_fn {
        return Ok(());
    }

    // Prepare page data as a Rhai map
    let mut page_map = rhai::Map::new();
    page_map.insert("title".into(), Dynamic::from(page.frontmatter.title.clone()));
    page_map.insert("slug".into(), Dynamic::from(page.slug.clone()));
    page_map.insert(
        "content".into(),
        Dynamic::from(page.raw_content.clone()),
    );

    if let Some(ref date) = page.frontmatter.date {
        page_map.insert("date".into(), Dynamic::from(date.clone()));
    }

    // Extra data
    let mut extra_map = rhai::Map::new();
    if let Some(ref extra) = page.frontmatter.extra {
        for (k, v) in extra {
            extra_map.insert(k.clone().into(), json_to_dynamic(v));
        }
    }
    page_map.insert("extra".into(), Dynamic::from(extra_map));

    // Call the hook function with page_map as argument
    let result: Dynamic = engine
        .call_fn(&mut Scope::new(), ast, hook_name, (page_map,))
        .map_err(|e| {
            anyhow::anyhow!(
                "Rhai plugin '{script_name}' error in {hook_name}: {e}"
            )
        })?;

    // The function should return the modified page map
    if let Some(updated) = result.try_cast::<rhai::Map>() {
        if let Some(title) = updated.get("title") {
            if let Some(s) = title.clone().into_string().ok() {
                page.frontmatter.title = s.into();
            }
        }

        if let Some(extra_val) = updated.get("extra") {
            if let Some(extra_map) = extra_val.clone().try_cast::<rhai::Map>() {
                let extra = page
                    .frontmatter
                    .extra
                    .get_or_insert_with(std::collections::HashMap::new);
                for (k, v) in extra_map {
                    extra.insert(k.to_string(), dynamic_to_json(&v));
                }
            }
        }
    }

    Ok(())
}

fn json_to_dynamic(v: &serde_json::Value) -> Dynamic {
    match v {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(a) => {
            let arr: Vec<Dynamic> = a.iter().map(json_to_dynamic).collect();
            Dynamic::from(arr)
        }
        serde_json::Value::Object(m) => {
            let mut map = rhai::Map::new();
            for (k, v) in m {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}

fn dynamic_to_json(v: &Dynamic) -> serde_json::Value {
    if v.is_unit() {
        serde_json::Value::Null
    } else if let Some(b) = v.as_bool().ok() {
        serde_json::Value::Bool(b)
    } else if let Some(i) = v.as_int().ok() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Some(f) = v.as_float().ok() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    } else if let Some(s) = v.clone().into_string().ok() {
        serde_json::Value::String(s)
    } else if let Some(arr) = v.clone().try_cast::<Vec<Dynamic>>() {
        serde_json::Value::Array(arr.iter().map(dynamic_to_json).collect())
    } else if let Some(map) = v.clone().try_cast::<rhai::Map>() {
        let obj: serde_json::Map<String, serde_json::Value> = map
            .iter()
            .map(|(k, v)| (k.to_string(), dynamic_to_json(v)))
            .collect();
        serde_json::Value::Object(obj)
    } else {
        serde_json::Value::String(v.to_string())
    }
}

/// Load all Rhai plugins from a directory.
pub fn load_rhai_plugins(plugins_dir: &Path) -> Result<Vec<Box<dyn Plugin>>> {
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

    if !plugins_dir.exists() {
        return Ok(plugins);
    }

    for entry in std::fs::read_dir(plugins_dir)
        .with_context(|| format!("Failed to read plugins dir: {}", plugins_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("rhai") {
            let plugin = RhaiPlugin::from_file(&path)?;
            plugins.push(Box::new(plugin));
        }
    }

    Ok(plugins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use crate::plugin::PluginManager;
    use std::path::PathBuf;

    fn test_page(content: &str) -> Page {
        Page {
            source_path: PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: Frontmatter {
                title: "Test Page".into(),
                ..Default::default()
            },
            raw_content: content.to_string(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    #[test]
    fn rhai_plugin_loads_and_executes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("test-plugin.rhai"),
            r#"
fn on_page_discovered(page) {
    page.extra.processed = true;
    page
}
"#,
        )
        .unwrap();

        let plugins = load_rhai_plugins(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name(), "test-plugin");
    }

    #[test]
    fn rhai_plugin_modifies_page_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("word-count.rhai"),
            r#"
fn on_page_discovered(page) {
    let words = page.content.split(' ');
    page.extra.word_count = words.len();
    page
}
"#,
        )
        .unwrap();

        let plugins = load_rhai_plugins(dir.path()).unwrap();
        let mut manager = PluginManager::new();
        for plugin in plugins {
            manager.register(plugin);
        }

        let mut page = test_page("one two three four five");
        manager.run_page_discovered(&mut page).unwrap();

        let extra = page.frontmatter.extra.unwrap();
        assert_eq!(extra["word_count"], serde_json::json!(5));
    }

    #[test]
    fn rhai_script_error_handling() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("bad.rhai"),
            "fn on_page_discovered(page) { let x = undefined_var; page }",
        )
        .unwrap();

        let plugins = load_rhai_plugins(dir.path()).unwrap();
        let mut page = test_page("content");
        let result = plugins[0].on_page_discovered(&mut page);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("bad"));
    }

    #[test]
    fn nonexistent_plugins_dir_returns_empty() {
        let plugins = load_rhai_plugins(Path::new("/nonexistent/plugins")).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn rhai_script_without_hook_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("empty.rhai"),
            "// This script has no hooks\nlet x = 42;",
        )
        .unwrap();

        let plugins = load_rhai_plugins(dir.path()).unwrap();
        let mut page = test_page("content");
        // Should not error, just skip
        plugins[0].on_page_discovered(&mut page).unwrap();
    }
}
