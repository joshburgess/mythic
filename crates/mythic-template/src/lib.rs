//! Template engine integration for Mythic, built on Tera.

use anyhow::{Context, Result};
use mythic_core::config::SiteConfig;
use mythic_core::page::Page;
use std::path::Path;
use tera::Tera;

/// Template rendering engine backed by Tera.
pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    /// Load all `.html` templates from the given directory.
    pub fn new(template_dir: &Path) -> Result<Self> {
        let glob = template_dir.join("**/*.html");
        let glob_str = glob.to_string_lossy();
        let tera = Tera::new(&glob_str)
            .with_context(|| format!("Failed to load templates from {}", template_dir.display()))?;
        Ok(Self { tera })
    }

    /// Render a page using its specified layout template.
    pub fn render(&self, page: &Page, config: &SiteConfig) -> Result<String> {
        let layout = page
            .frontmatter
            .layout
            .as_deref()
            .unwrap_or("default");
        let template_name = format!("{}.html", layout);

        let mut ctx = tera::Context::new();

        // Page context
        ctx.insert("page", &page.frontmatter);
        ctx.insert("content", page.rendered_html.as_deref().unwrap_or(""));

        // Site context
        let mut site = std::collections::HashMap::new();
        site.insert("title", config.title.as_str());
        site.insert("base_url", config.base_url.as_str());
        ctx.insert("site", &site);

        self.tera
            .render(&template_name, &ctx)
            .with_context(|| format!("Failed to render template '{}' for page '{}'", template_name, page.slug))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mythic_core::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn test_page() -> Page {
        Page {
            source_path: PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: Frontmatter {
                title: "Test Page".to_string(),
                layout: Some("default".to_string()),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: Some("<p>Hello world</p>".to_string()),
            output_path: None,
            content_hash: 0,
        }
    }

    fn test_config() -> SiteConfig {
        SiteConfig {
            title: "My Site".to_string(),
            base_url: "http://localhost:3000".to_string(),
            content_dir: PathBuf::from("content"),
            output_dir: PathBuf::from("public"),
            template_dir: PathBuf::from("templates"),
            data_dir: PathBuf::from("_data"),
        }
    }

    #[test]
    fn render_default_layout() {
        let engine = TemplateEngine::new(Path::new("../../fixtures/basic-site/templates")).unwrap();
        let page = test_page();
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains("<p>Hello world</p>"));
        assert!(html.contains("Test Page"));
    }

    #[test]
    fn missing_template_errors() {
        let engine = TemplateEngine::new(Path::new("../../fixtures/basic-site/templates")).unwrap();
        let mut page = test_page();
        page.frontmatter.layout = Some("nonexistent".to_string());
        let config = test_config();
        assert!(engine.render(&page, &config).is_err());
    }
}
