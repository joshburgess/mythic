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
        let mut tera = tera::Tera::new(&html_glob).unwrap_or_default();

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

                    hbs.register_template_file(&rel, path).with_context(|| {
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
        self.render_full(page, config, assets, None)
    }

    /// Render a page with full context: assets and site data.
    pub fn render_full(
        &self,
        page: &Page,
        config: &SiteConfig,
        assets: Option<&serde_json::Value>,
        data: Option<&serde_json::Value>,
    ) -> Result<String> {
        let layout = page.frontmatter.layout.as_deref().unwrap_or("default");

        let engine = self
            .layout_engines
            .get(layout)
            .map(|s| s.as_str())
            .unwrap_or(&self.default_engine);

        match engine {
            "hbs" | "handlebars" => self.render_hbs(page, config, layout, assets, data),
            _ => self.render_tera(page, config, layout, assets, data),
        }
    }

    fn render_tera(
        &self,
        page: &Page,
        config: &SiteConfig,
        layout: &str,
        assets: Option<&serde_json::Value>,
        data: Option<&serde_json::Value>,
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

        if let Some(data) = data {
            ctx.insert("data", data);
        }

        self.tera.render(&template_name, &ctx).with_context(|| {
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
        site_data: Option<&serde_json::Value>,
    ) -> Result<String> {
        let template_name = format!("{layout}.hbs");

        let mut data = serde_json::Map::new();
        data.insert("page".to_string(), serde_json::to_value(&page.frontmatter)?);
        data.insert(
            "content".to_string(),
            serde_json::Value::String(page.rendered_html.as_deref().unwrap_or("").to_string()),
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

        if let Some(site_data) = site_data {
            data.insert("data".to_string(), site_data.clone());
        }

        self.hbs.render(&template_name, &data).with_context(|| {
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
                title: "Test Page".into(),
                layout: Some(layout.into()),
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
        let engine = TemplateEngine::new(Path::new("../../fixtures/basic-site/templates")).unwrap();
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
        let html = engine
            .render(&test_page("default"), &test_config())
            .unwrap();
        assert!(html.contains("Test Page"));
    }

    #[test]
    fn missing_template_errors() {
        let engine = TemplateEngine::new(Path::new("../../fixtures/basic-site/templates")).unwrap();
        let page = test_page("nonexistent");
        assert!(engine.render(&page, &test_config()).is_err());
    }

    // --- Comprehensive template rendering tests ---

    /// Helper: create a Page with all context variables populated.
    fn full_page(layout: &str) -> Page {
        Page {
            source_path: PathBuf::from("blog/my-post.md"),
            slug: "my-post".to_string(),
            frontmatter: Frontmatter {
                title: "Full Page Title".into(),
                date: Some("2025-06-15".into()),
                draft: Some(false),
                layout: Some(layout.into()),
                tags: Some(vec!["rust".into(), "web".into(), "ssg".into()]),
                extra: None,
                sitemap: Some(true),
                locale: None,
                aliases: None,
            },
            raw_content: "# Hello".to_string(),
            rendered_html: Some("<h1>Hello</h1><p>World</p>".to_string()),
            output_path: None,
            content_hash: 42,
            toc: vec![
                mythic_core::page::TocEntry {
                    level: 1,
                    text: "Hello".to_string(),
                    id: "hello".to_string(),
                },
                mythic_core::page::TocEntry {
                    level: 2,
                    text: "Sub Section".to_string(),
                    id: "sub-section".to_string(),
                },
            ],
        }
    }

    #[test]
    fn tera_all_context_variables() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("post.html"),
            concat!(
                "<html><head><title>{{ page.title }} | {{ site.title }}</title></head>",
                "<body>",
                "<p>Date: {{ page.date }}</p>",
                "<p>Tags: {% for tag in page.tags %}{{ tag }}{% if not loop.last %}, {% endif %}{% endfor %}</p>",
                "<p>Base: {{ site.base_url | safe }}</p>",
                "<div>{{ content | safe }}</div>",
                "{% for entry in toc %}<a href=\"#{{ entry.id }}\">{{ entry.text }}</a>{% endfor %}",
                "</body></html>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = full_page("post");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("Full Page Title | My Site"));
        assert!(html.contains("Date: 2025-06-15"));
        assert!(html.contains("rust, web, ssg"));
        assert!(html.contains("Base: http://localhost:3000"));
        assert!(html.contains("<h1>Hello</h1><p>World</p>"));
        assert!(html.contains("href=\"#hello\""));
        assert!(html.contains("Sub Section"));
    }

    #[test]
    fn tera_template_inheritance() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("base.html"),
            concat!(
                "<!DOCTYPE html><html><head><title>{% block title %}Default{% endblock %}</title></head>",
                "<body>{% block body %}{% endblock %}</body></html>",
            ),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("child.html"),
            concat!(
                "{% extends \"base.html\" %}",
                "{% block title %}{{ page.title }}{% endblock %}",
                "{% block body %}<article>{{ content | safe }}</article>{% endblock %}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("child");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Test Page</title>"));
        assert!(html.contains("<article><p>Hello world</p></article>"));
    }

    #[test]
    fn tera_includes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("header.html"),
            "<header>{{ site.title }}</header>",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("withinclude.html"),
            concat!(
                "{% include \"header.html\" %}",
                "<main>{{ content | safe }}</main>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("withinclude");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<header>My Site</header>"));
        assert!(html.contains("<main><p>Hello world</p></main>"));
    }

    #[test]
    fn tera_filters() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("filters.html"),
            concat!(
                "<p>{{ content | safe }}</p>",
                "<p>{{ page.title | upper }}</p>",
                "<p>{{ page.title | lower }}</p>",
                "{% if page.tags %}<p>{{ page.tags | length }}</p>{% endif %}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = full_page("filters");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<h1>Hello</h1><p>World</p>"));
        assert!(html.contains("FULL PAGE TITLE"));
        assert!(html.contains("full page title"));
        assert!(html.contains("<p>3</p>"));
    }

    #[test]
    fn tera_for_loop_over_tags() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("taglist.html"),
            concat!(
                "<ul>",
                "{% for tag in page.tags %}<li>{{ tag }}</li>{% endfor %}",
                "</ul>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = full_page("taglist");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<li>rust</li>"));
        assert!(html.contains("<li>web</li>"));
        assert!(html.contains("<li>ssg</li>"));
    }

    #[test]
    fn tera_if_else_conditionals() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("cond.html"),
            concat!(
                "{% if page.date %}<time>{{ page.date }}</time>",
                "{% else %}<span>No date</span>{% endif %}",
                "{% if page.draft %}<span>DRAFT</span>",
                "{% else %}<span>PUBLISHED</span>{% endif %}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let config = test_config();

        // Page with date and draft=false
        let page = full_page("cond");
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains("<time>2025-06-15</time>"));
        assert!(html.contains("PUBLISHED"));

        // Page without date
        let mut page_no_date = test_page("cond");
        page_no_date.frontmatter.date = None;
        let html2 = engine.render(&page_no_date, &config).unwrap();
        assert!(html2.contains("<span>No date</span>"));
    }

    #[test]
    fn hbs_triple_stash_raw_html() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("raw.hbs"),
            "<div>{{page.title}}</div><div>{{{content}}}</div>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("raw");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        // Triple-stash should preserve raw HTML without escaping
        assert!(html.contains("<p>Hello world</p>"));
        assert!(html.contains("<div>Test Page</div>"));
    }

    #[test]
    fn hbs_helpers_if_each_unless() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("helpers.hbs"),
            concat!(
                "{{#if page.date}}<time>{{page.date}}</time>{{else}}<span>No date</span>{{/if}}",
                "{{#each page.tags}}<span class=\"tag\">{{this}}</span>{{/each}}",
                "{{#unless page.draft}}<span>live</span>{{/unless}}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = full_page("helpers");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<time>2025-06-15</time>"));
        assert!(html.contains("<span class=\"tag\">rust</span>"));
        assert!(html.contains("<span class=\"tag\">web</span>"));
        assert!(html.contains("<span class=\"tag\">ssg</span>"));
        assert!(html.contains("<span>live</span>"));
    }

    #[test]
    fn hbs_site_context() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("siteinfo.hbs"),
            "<h1>{{site.title}}</h1><a href=\"{{site.base_url}}\">Home</a>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("siteinfo");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<h1>My Site</h1>"));
        assert!(html.contains("href=\"http://localhost:3000\""));
    }

    #[test]
    fn tera_assets_context() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("withassets.html"),
            concat!(
                "<link rel=\"stylesheet\" href=\"{{ assets.css_path | safe }}\">",
                "<script src=\"{{ assets.js_path | safe }}\"></script>",
                "<div>{{ content | safe }}</div>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("withassets");
        let config = test_config();

        let assets = serde_json::json!({
            "css_path": "/assets/style.abc123.css",
            "js_path": "/assets/app.def456.js"
        });

        let html = engine
            .render_with_assets(&page, &config, Some(&assets))
            .unwrap();

        assert!(html.contains("href=\"/assets/style.abc123.css\""));
        assert!(html.contains("src=\"/assets/app.def456.js\""));
        assert!(html.contains("<p>Hello world</p>"));
    }

    #[test]
    fn hbs_assets_context() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("withassets.hbs"),
            concat!(
                "<link rel=\"stylesheet\" href=\"{{assets.css_path}}\">",
                "<script src=\"{{assets.js_path}}\"></script>",
                "<div>{{{content}}}</div>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("withassets");
        let config = test_config();

        let assets = serde_json::json!({
            "css_path": "/assets/style.abc123.css",
            "js_path": "/assets/app.def456.js"
        });

        let html = engine
            .render_with_assets(&page, &config, Some(&assets))
            .unwrap();

        assert!(html.contains("href=\"/assets/style.abc123.css\""));
        assert!(html.contains("src=\"/assets/app.def456.js\""));
    }

    #[test]
    fn empty_content_renders_without_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("empty.html"),
            "<div>{{ content | safe }}</div>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("empty");
        page.rendered_html = Some(String::new());
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<div></div>"));
    }

    #[test]
    fn none_content_renders_without_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("nohtml.html"),
            "<div>{{ content | safe }}</div>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("nohtml");
        page.rendered_html = None;
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<div></div>"));
    }

    #[test]
    fn page_with_no_date_renders_without_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("nodate.html"),
            concat!(
                "{% if page.date %}<time>{{ page.date }}</time>{% endif %}",
                "<h1>{{ page.title }}</h1>",
                "<div>{{ content | safe }}</div>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("nodate");
        page.frontmatter.date = None;
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(!html.contains("<time>"));
        assert!(html.contains("<h1>Test Page</h1>"));
    }

    #[test]
    fn page_with_extra_data_accessible_in_tera_template() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("extra.html"),
            concat!(
                "<h1>{{ page.title }}</h1>",
                "{% if page.extra.author %}<p>By {{ page.extra.author }}</p>{% endif %}",
                "{% if page.extra.reading_time %}<span>{{ page.extra.reading_time }} min read</span>{% endif %}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("extra");
        let mut extra = HashMap::new();
        extra.insert(
            "author".to_string(),
            serde_json::Value::String("Jane Doe".to_string()),
        );
        extra.insert(
            "reading_time".to_string(),
            serde_json::Value::Number(serde_json::Number::from(5)),
        );
        page.frontmatter.extra = Some(extra);
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("By Jane Doe"));
        assert!(html.contains("5 min read"));
    }

    #[test]
    fn page_with_extra_data_accessible_in_hbs_template() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("extra.hbs"),
            concat!(
                "<h1>{{page.title}}</h1>",
                "{{#if page.extra.author}}<p>By {{page.extra.author}}</p>{{/if}}",
                "{{#if page.extra.reading_time}}<span>{{page.extra.reading_time}} min read</span>{{/if}}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("extra");
        let mut extra = HashMap::new();
        extra.insert(
            "author".to_string(),
            serde_json::Value::String("Jane Doe".to_string()),
        );
        extra.insert(
            "reading_time".to_string(),
            serde_json::Value::Number(serde_json::Number::from(5)),
        );
        page.frontmatter.extra = Some(extra);
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("By Jane Doe"));
        assert!(html.contains("5 min read"));
    }

    #[test]
    fn tera_template_with_toc_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("withtoc.html"),
            concat!(
                "<nav>",
                "{% for entry in toc %}",
                "<a href=\"#{{ entry.id }}\" class=\"toc-h{{ entry.level }}\">{{ entry.text }}</a>",
                "{% endfor %}",
                "</nav>",
                "<div>{{ content | safe }}</div>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = full_page("withtoc");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("href=\"#hello\""));
        assert!(html.contains("class=\"toc-h1\""));
        assert!(html.contains(">Hello</a>"));
        assert!(html.contains("href=\"#sub-section\""));
        assert!(html.contains("class=\"toc-h2\""));
        assert!(html.contains(">Sub Section</a>"));
    }

    #[test]
    fn hbs_template_with_toc_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("withtoc.hbs"),
            concat!(
                "<nav>",
                "{{#each toc}}",
                "<a href=\"#{{this.id}}\" class=\"toc-h{{this.level}}\">{{this.text}}</a>",
                "{{/each}}",
                "</nav>",
                "<div>{{{content}}}</div>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = full_page("withtoc");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("href=\"#hello\""));
        assert!(html.contains("class=\"toc-h1\""));
        assert!(html.contains(">Hello</a>"));
        assert!(html.contains("href=\"#sub-section\""));
        assert!(html.contains("class=\"toc-h2\""));
        assert!(html.contains(">Sub Section</a>"));
    }

    #[test]
    fn empty_toc_renders_without_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("emptytoc.html"),
            concat!(
                "{% if toc %}<nav>",
                "{% for entry in toc %}<a href=\"#{{ entry.id }}\">{{ entry.text }}</a>{% endfor %}",
                "</nav>{% endif %}",
                "<div>{{ content | safe }}</div>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("emptytoc"); // test_page has empty toc
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        // Empty vec is falsy in Tera, so nav should not appear
        assert!(html.contains("<p>Hello world</p>"));
    }
}
