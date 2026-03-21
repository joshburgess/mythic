//! Multi-engine template system for Mythic.
//!
//! Supports Tera (.html, .tera) and Handlebars (.hbs) templates.
//! Templates can be mixed within a single project.

use anyhow::{Context, Result};
use mythic_core::config::SiteConfig;
use mythic_core::page::Page;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Multi-engine template renderer.
pub struct TemplateEngine {
    tera: tera::Tera,
    hbs: handlebars::Handlebars<'static>,
    /// Maps layout name → engine ("tera" or "hbs")
    layout_engines: HashMap<String, String>,
    default_engine: String,
}

impl TemplateEngine {
    /// Load all templates from the given directory.
    pub fn new(template_dir: &Path) -> Result<Self> {
        Self::new_with_default(template_dir, "tera")
    }

    /// Load templates with a specified default engine for .html files.
    pub fn new_with_default(template_dir: &Path, default_engine: &str) -> Result<Self> {
        let mut layout_engines = HashMap::new();

        // Load Tera templates (.html and .tera)
        let html_glob = template_dir.join("**/*.html").to_string_lossy().to_string();
        let mut tera = match tera::Tera::new(&html_glob) {
            Ok(t) => t,
            Err(_) => tera::Tera::default(),
        };

        // Also load .tera files by reading them manually
        if template_dir.exists() {
            for entry in WalkDir::new(template_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let rel = path
                    .strip_prefix(template_dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                match ext {
                    "tera" => {
                        let content = std::fs::read_to_string(path)?;
                        tera.add_raw_template(&rel, &content).ok();
                        let layout_name = rel.trim_end_matches(".tera").to_string();
                        layout_engines.insert(layout_name, "tera".to_string());
                    }
                    "html" => {
                        let layout_name = rel.trim_end_matches(".html").to_string();
                        layout_engines.insert(layout_name, default_engine.to_string());
                    }
                    _ => {}
                }
            }
        }

        // Load Handlebars templates (.hbs)
        let mut hbs = handlebars::Handlebars::new();
        hbs.set_strict_mode(true);

        if template_dir.exists() {
            for entry in WalkDir::new(template_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                if ext == "hbs" {
                    let rel = path
                        .strip_prefix(template_dir)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();

                    hbs.register_template_file(&rel, path)
                        .with_context(|| {
                            format!("Failed to load Handlebars template: {}", path.display())
                        })?;

                    let layout_name = rel.trim_end_matches(".hbs").to_string();
                    layout_engines.insert(layout_name, "hbs".to_string());
                }
            }
        }

        Ok(Self {
            tera,
            hbs,
            layout_engines,
            default_engine: default_engine.to_string(),
        })
    }

    /// Render a page using its specified layout template.
    pub fn render(&self, page: &Page, config: &SiteConfig) -> Result<String> {
        self.render_with_assets(page, config, None)
    }

    /// Render a page with optional asset manifest context.
    pub fn render_with_assets(
        &self,
        page: &Page,
        config: &SiteConfig,
        assets: Option<&serde_json::Value>,
    ) -> Result<String> {
        let layout = page
            .frontmatter
            .layout
            .as_deref()
            .unwrap_or("default");

        let engine = self
            .layout_engines
            .get(layout)
            .map(|s| s.as_str())
            .unwrap_or(&self.default_engine);

        match engine {
            "hbs" | "handlebars" => self.render_hbs(page, config, layout, assets),
            _ => self.render_tera(page, config, layout, assets),
        }
    }

    fn render_tera(
        &self,
        page: &Page,
        config: &SiteConfig,
        layout: &str,
        assets: Option<&serde_json::Value>,
    ) -> Result<String> {
        let template_name = if self.tera.get_template(&format!("{layout}.html")).is_ok() {
            format!("{layout}.html")
        } else if self.tera.get_template(&format!("{layout}.tera")).is_ok() {
            format!("{layout}.tera")
        } else {
            format!("{layout}.html")
        };

        let mut ctx = tera::Context::new();
        ctx.insert("page", &page.frontmatter);
        ctx.insert("content", page.rendered_html.as_deref().unwrap_or(""));
        ctx.insert("toc", &page.toc);

        let mut site = HashMap::new();
        site.insert("title", config.title.as_str());
        site.insert("base_url", config.base_url.as_str());
        ctx.insert("site", &site);

        if let Some(assets) = assets {
            ctx.insert("assets", assets);
        }

        self.tera
            .render(&template_name, &ctx)
            .with_context(|| {
                format!(
                    "Failed to render Tera template '{template_name}' for '{}'",
                    page.slug
                )
            })
    }

    fn render_hbs(
        &self,
        page: &Page,
        config: &SiteConfig,
        layout: &str,
        assets: Option<&serde_json::Value>,
    ) -> Result<String> {
        let template_name = format!("{layout}.hbs");

        let mut data = serde_json::Map::new();
        data.insert(
            "page".to_string(),
            serde_json::to_value(&page.frontmatter)?,
        );
        data.insert(
            "content".to_string(),
            serde_json::Value::String(
                page.rendered_html.as_deref().unwrap_or("").to_string(),
            ),
        );
        data.insert("toc".to_string(), serde_json::to_value(&page.toc)?);

        let mut site = serde_json::Map::new();
        site.insert(
            "title".to_string(),
            serde_json::Value::String(config.title.clone()),
        );
        site.insert(
            "base_url".to_string(),
            serde_json::Value::String(config.base_url.clone()),
        );
        data.insert("site".to_string(), serde_json::Value::Object(site));

        if let Some(assets) = assets {
            data.insert("assets".to_string(), assets.clone());
        }

        self.hbs
            .render(&template_name, &data)
            .with_context(|| {
                format!(
                    "Failed to render Handlebars template '{template_name}' for '{}'",
                    page.slug
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mythic_core::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn test_page(layout: &str) -> Page {
        Page {
            source_path: PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: Frontmatter {
                title: "Test Page".to_string(),
                layout: Some(layout.to_string()),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: Some("<p>Hello world</p>".to_string()),
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    fn test_config() -> SiteConfig {
        SiteConfig::for_testing("My Site", "http://localhost:3000")
    }

    #[test]
    fn tera_rendering() {
        let engine =
            TemplateEngine::new(Path::new("../../fixtures/basic-site/templates")).unwrap();
        let page = test_page("default");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains("<p>Hello world</p>"));
        assert!(html.contains("Test Page"));
    }

    #[test]
    fn handlebars_rendering() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("page.hbs"),
            "<html><body><h1>{{page.title}}</h1>{{{content}}}</body></html>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("page");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains("Test Page"));
        assert!(html.contains("<p>Hello world</p>"));
    }

    #[test]
    fn mixed_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("default.html"),
            "<html><body>{{ page.title }} — {{ content | safe }}</body></html>",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("blog.hbs"),
            "<article><h1>{{page.title}}</h1>{{{content}}}</article>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let config = test_config();

        let tera_html = engine.render(&test_page("default"), &config).unwrap();
        assert!(tera_html.contains("Test Page"));

        let hbs_html = engine.render(&test_page("blog"), &config).unwrap();
        assert!(hbs_html.contains("<article>"));
        assert!(hbs_html.contains("Test Page"));
    }

    #[test]
    fn default_engine_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("default.html"),
            "<html>{{ page.title }} — {{ content | safe }}</html>",
        )
        .unwrap();

        let engine = TemplateEngine::new_with_default(dir.path(), "tera").unwrap();
        let html = engine.render(&test_page("default"), &test_config()).unwrap();
        assert!(html.contains("Test Page"));
    }

    #[test]
    fn missing_template_errors() {
        let engine =
            TemplateEngine::new(Path::new("../../fixtures/basic-site/templates")).unwrap();
        let page = test_page("nonexistent");
        assert!(engine.render(&page, &test_config()).is_err());
    }
}
