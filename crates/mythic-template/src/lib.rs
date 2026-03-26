//! Multi-engine template system for Mythic.
//!
//! Supports Tera (.html, .tera), Handlebars (.hbs), and MiniJinja (.jinja, .j2, .jinja2) templates.
//! Templates can be mixed within a single project.

use anyhow::{Context, Result};
use mythic_core::config::SiteConfig;
use mythic_core::page::Page;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

const KATEX_CSS: &str = "https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css";
const KATEX_JS: &str = "https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js";
const KATEX_AUTO_RENDER_JS: &str = "https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/contrib/auto-render.min.js";

// --- Shared pure filter functions ---

/// Compute reading time from text: "N min read"
fn compute_reading_time(text: &str) -> String {
    let words = text.split_whitespace().count();
    let minutes = words.div_ceil(200);
    if minutes <= 1 {
        "1 min read".to_string()
    } else {
        format!("{minutes} min read")
    }
}

/// Compute word count from text.
fn compute_word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Truncate text to `count` words, appending "..." if truncated.
fn compute_truncate_words(text: &str, count: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= count {
        text.to_string()
    } else {
        let truncated = words[..count].join(" ");
        format!("{truncated}...")
    }
}

/// Render markdown to HTML inline, stripping wrapping `<p>` for single-paragraph input.
fn compute_markdownify(text: &str) -> String {
    let mut opts = pulldown_cmark::Options::empty();
    opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    let parser = pulldown_cmark::Parser::new_ext(text, opts);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    let trimmed = html.trim();
    if trimmed.starts_with("<p>")
        && trimmed.ends_with("</p>")
        && trimmed.matches("<p>").count() == 1
    {
        trimmed[3..trimmed.len() - 4].to_string()
    } else {
        html
    }
}

/// Strip HTML tags, return plain text.
fn compute_plainify(text: &str) -> String {
    let mut plain = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            plain.push(c);
        }
    }
    plain
}

/// Convert slug to title case: "my-slug" -> "My Slug"
fn compute_humanize(text: &str) -> String {
    text.replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.collect::<String>()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Naive pluralization.
fn compute_pluralize(text: &str) -> String {
    if text.ends_with('s')
        || text.ends_with("sh")
        || text.ends_with("ch")
        || text.ends_with('x')
    {
        format!("{text}es")
    } else if text.ends_with('y')
        && !text.ends_with("ey")
        && !text.ends_with("ay")
        && !text.ends_with("oy")
    {
        format!("{}ies", &text[..text.len() - 1])
    } else {
        format!("{text}s")
    }
}

/// Naive singularization.
fn compute_singularize(text: &str) -> String {
    if text.ends_with("ies") {
        format!("{}y", &text[..text.len() - 3])
    } else if text.ends_with("ses")
        || text.ends_with("shes")
        || text.ends_with("ches")
        || text.ends_with("xes")
    {
        text[..text.len() - 2].to_string()
    } else if text.ends_with('s') && !text.ends_with("ss") {
        text[..text.len() - 1].to_string()
    } else {
        text.to_string()
    }
}

/// Convert text to URL-friendly slug: "My Title" -> "my-title"
fn compute_urlize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Multi-engine template renderer.
pub struct TemplateEngine {
    tera: tera::Tera,
    hbs: handlebars::Handlebars<'static>,
    mj: minijinja::Environment<'static>,
    /// Maps layout name -> engine ("tera", "hbs", or "minijinja")
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
        // Disable Tera's default HTML auto-escaping. We're rendering HTML
        // templates, not user input, so auto-escaping breaks intentional HTML
        // in variables like `content`. This also makes Hugo-compat filters
        // like `safeHTML` work as expected without requiring `| safe`.
        tera.autoescape_on(vec![]);

        // MiniJinja environment
        let mut mj = minijinja::Environment::new();
        mj.set_auto_escape_callback(|_| minijinja::AutoEscape::None);

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
                        // If default engine is minijinja, also load .html into MiniJinja
                        if default_engine == "minijinja" {
                            let content = std::fs::read_to_string(path)?;
                            mj.add_template_owned(rel.clone(), content).ok();
                        }
                    }
                    "jinja" | "j2" | "jinja2" => {
                        let content = std::fs::read_to_string(path)?;
                        mj.add_template_owned(rel.clone(), content)
                            .with_context(|| {
                                format!(
                                    "Failed to load MiniJinja template: {}",
                                    path.display()
                                )
                            })?;
                        let layout_name = rel
                            .trim_end_matches(".jinja")
                            .trim_end_matches(".j2")
                            .trim_end_matches(".jinja2")
                            .to_string();
                        layout_engines.insert(layout_name, "minijinja".to_string());
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

        // Register custom Tera filters
        tera.register_filter("reading_time", reading_time_filter);
        tera.register_filter("word_count", word_count_filter);
        tera.register_filter("truncate_words", truncate_words_filter);
        // Hugo compatibility filters
        tera.register_filter("markdownify", markdownify_filter);
        tera.register_filter("plainify", plainify_filter);
        tera.register_filter("humanize", humanize_filter);
        tera.register_filter("pluralize", pluralize_filter);
        tera.register_filter("singularize", singularize_filter);
        tera.register_filter("urlize", urlize_filter);
        tera.register_filter("safeHTML", safe_html_filter);
        tera.register_filter("safeCSS", safe_html_filter);
        tera.register_filter("safeJS", safe_html_filter);

        // Register custom MiniJinja filters
        mj.add_filter("reading_time", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_reading_time(&value))
        });
        mj.add_filter("word_count", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_word_count(&value).to_string())
        });
        mj.add_filter("truncate_words", |value: String, kwargs: minijinja::value::Kwargs| -> Result<String, minijinja::Error> {
            let count: usize = kwargs.get::<usize>("count").unwrap_or(20);
            Ok(compute_truncate_words(&value, count))
        });
        mj.add_filter("markdownify", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_markdownify(&value))
        });
        mj.add_filter("plainify", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_plainify(&value))
        });
        mj.add_filter("humanize", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_humanize(&value))
        });
        mj.add_filter("pluralize", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_pluralize(&value))
        });
        mj.add_filter("singularize", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_singularize(&value))
        });
        mj.add_filter("urlize", |value: String| -> Result<String, minijinja::Error> {
            Ok(compute_urlize(&value))
        });
        mj.add_filter("safeHTML", |value: String| -> Result<String, minijinja::Error> {
            Ok(value)
        });
        mj.add_filter("safeCSS", |value: String| -> Result<String, minijinja::Error> {
            Ok(value)
        });
        mj.add_filter("safeJS", |value: String| -> Result<String, minijinja::Error> {
            Ok(value)
        });

        Ok(Self {
            tera,
            hbs,
            mj,
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
            "minijinja" | "jinja" => self.render_minijinja(page, config, layout, assets, data),
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
        // Build page context from frontmatter plus slug and url
        let rendered_html = page.rendered_html.as_deref().unwrap_or("");
        let has_math = rendered_html.contains("class=\"math ");

        let mut page_ctx = serde_json::to_value(&page.frontmatter).unwrap_or_default();
        if let serde_json::Value::Object(ref mut map) = page_ctx {
            map.insert(
                "slug".to_string(),
                serde_json::Value::String(page.slug.clone()),
            );
            map.insert(
                "url".to_string(),
                serde_json::Value::String(format!("{}/{}/", config.base_path(), page.slug)),
            );
            map.insert(
                "has_math".to_string(),
                serde_json::Value::Bool(has_math),
            );
        }
        ctx.insert("page", &page_ctx);
        ctx.insert("content", rendered_html);
        ctx.insert("toc", &page.toc);

        let mut site = HashMap::new();
        site.insert("title", config.title.as_str());
        site.insert("base_url", config.base_url.as_str());
        site.insert("base_path", config.base_path());
        ctx.insert("site", &site);

        ctx.insert("katex_css", KATEX_CSS);
        ctx.insert("katex_js", KATEX_JS);
        ctx.insert("katex_auto_render_js", KATEX_AUTO_RENDER_JS);

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

        let rendered_html = page.rendered_html.as_deref().unwrap_or("");
        let has_math = rendered_html.contains("class=\"math ");

        let mut data = serde_json::Map::new();
        // Build page context from frontmatter plus slug and url
        let mut page_ctx = serde_json::to_value(&page.frontmatter)?;
        if let serde_json::Value::Object(ref mut map) = page_ctx {
            map.insert(
                "slug".to_string(),
                serde_json::Value::String(page.slug.clone()),
            );
            map.insert(
                "url".to_string(),
                serde_json::Value::String(format!("{}/{}/", config.base_path(), page.slug)),
            );
            map.insert(
                "has_math".to_string(),
                serde_json::Value::Bool(has_math),
            );
        }
        data.insert("page".to_string(), page_ctx);
        data.insert(
            "content".to_string(),
            serde_json::Value::String(rendered_html.to_string()),
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
        site.insert(
            "base_path".to_string(),
            serde_json::Value::String(config.base_path().to_string()),
        );
        data.insert("site".to_string(), serde_json::Value::Object(site));

        data.insert("katex_css".to_string(), serde_json::Value::String(KATEX_CSS.to_string()));
        data.insert("katex_js".to_string(), serde_json::Value::String(KATEX_JS.to_string()));
        data.insert("katex_auto_render_js".to_string(), serde_json::Value::String(KATEX_AUTO_RENDER_JS.to_string()));

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

    fn render_minijinja(
        &self,
        page: &Page,
        config: &SiteConfig,
        layout: &str,
        assets: Option<&serde_json::Value>,
        site_data: Option<&serde_json::Value>,
    ) -> Result<String> {
        // Resolve template name: try .jinja, .j2, .jinja2, .html in order
        let extensions = [".jinja", ".j2", ".jinja2", ".html"];
        let template_name = extensions
            .iter()
            .map(|ext| format!("{layout}{ext}"))
            .find(|name| self.mj.get_template(name).is_ok())
            .unwrap_or_else(|| format!("{layout}.jinja"));

        let rendered_html = page.rendered_html.as_deref().unwrap_or("");
        let has_math = rendered_html.contains("class=\"math ");

        let mut data = serde_json::Map::new();

        // Build page context from frontmatter plus slug and url
        let mut page_ctx = serde_json::to_value(&page.frontmatter)?;
        if let serde_json::Value::Object(ref mut map) = page_ctx {
            map.insert(
                "slug".to_string(),
                serde_json::Value::String(page.slug.clone()),
            );
            map.insert(
                "url".to_string(),
                serde_json::Value::String(format!("{}/{}/", config.base_path(), page.slug)),
            );
            map.insert(
                "has_math".to_string(),
                serde_json::Value::Bool(has_math),
            );
        }
        data.insert("page".to_string(), page_ctx);
        data.insert(
            "content".to_string(),
            serde_json::Value::String(rendered_html.to_string()),
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
        site.insert(
            "base_path".to_string(),
            serde_json::Value::String(config.base_path().to_string()),
        );
        data.insert("site".to_string(), serde_json::Value::Object(site));

        data.insert("katex_css".to_string(), serde_json::Value::String(KATEX_CSS.to_string()));
        data.insert("katex_js".to_string(), serde_json::Value::String(KATEX_JS.to_string()));
        data.insert("katex_auto_render_js".to_string(), serde_json::Value::String(KATEX_AUTO_RENDER_JS.to_string()));

        if let Some(assets) = assets {
            data.insert("assets".to_string(), assets.clone());
        }

        if let Some(site_data) = site_data {
            data.insert("data".to_string(), site_data.clone());
        }

        let ctx = minijinja::value::Value::from_serialize(&data);

        let tmpl = self.mj.get_template(&template_name).with_context(|| {
            format!(
                "Failed to find MiniJinja template '{template_name}' for '{}'",
                page.slug
            )
        })?;

        tmpl.render(ctx).with_context(|| {
            format!(
                "Failed to render MiniJinja template '{template_name}' for '{}'",
                page.slug
            )
        })
    }
}

// --- Custom Tera filters (delegating to shared functions) ---

/// Filter: `{{ content | reading_time }}` -> "3 min read"
fn reading_time_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("reading_time", "value", String, value);
    Ok(tera::Value::String(compute_reading_time(&text)))
}

/// Filter: `{{ content | word_count }}` -> 342
fn word_count_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("word_count", "value", String, value);
    Ok(tera::Value::Number(serde_json::Number::from(compute_word_count(&text))))
}

/// Filter: `{{ content | truncate_words(count=20) }}` -> first 20 words + "..."
fn truncate_words_filter(
    value: &tera::Value,
    args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("truncate_words", "value", String, value);
    let count = match args.get("count") {
        Some(v) => v.as_u64().unwrap_or(20) as usize,
        None => 20,
    };
    Ok(tera::Value::String(compute_truncate_words(&text, count)))
}

// --- Hugo compatibility filters ---

/// Filter: `{{ text | markdownify }}` -- render markdown to HTML inline.
fn markdownify_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("markdownify", "value", String, value);
    Ok(tera::Value::String(compute_markdownify(&text)))
}

/// Filter: `{{ html | plainify }}` -- strip HTML tags, return plain text.
fn plainify_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("plainify", "value", String, value);
    Ok(tera::Value::String(compute_plainify(&text)))
}

/// Filter: `{{ "my-slug" | humanize }}` -> "My Slug"
fn humanize_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("humanize", "value", String, value);
    Ok(tera::Value::String(compute_humanize(&text)))
}

/// Filter: `{{ "post" | pluralize }}` -> "posts"
fn pluralize_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("pluralize", "value", String, value);
    Ok(tera::Value::String(compute_pluralize(&text)))
}

/// Filter: `{{ "posts" | singularize }}` -> "post"
fn singularize_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("singularize", "value", String, value);
    Ok(tera::Value::String(compute_singularize(&text)))
}

/// Filter: `{{ "My Title" | urlize }}` -> "my-title"
fn urlize_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let text = tera::try_get_value!("urlize", "value", String, value);
    Ok(tera::Value::String(compute_urlize(&text)))
}

/// Filter: `{{ html | safeHTML }}` -- Hugo compatibility alias.
/// With auto-escaping disabled (see `tera.autoescape_on(vec![])` in `new_with_default`),
/// this is a no-op pass-through, matching Hugo's behavior where `safeHTML` marks content
/// as safe for HTML output. The `| safe` built-in also works, but this filter exists so
/// Hugo templates using `| safeHTML` don't need to be rewritten.
fn safe_html_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    Ok(value.clone())
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

    // --- Custom filter tests ---

    #[test]
    fn reading_time_filter_works() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("rt.html"), "{{ content | reading_time }}").unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("rt");
        // ~400 words -> 2 min read
        page.rendered_html = Some(vec!["word"; 400].join(" "));
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains("2 min read"), "Got: {html}");
    }

    #[test]
    fn reading_time_short_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("rt.html"), "{{ content | reading_time }}").unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("rt");
        page.rendered_html = Some("short".to_string());
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains("1 min read"));
    }

    #[test]
    fn word_count_filter_works() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("wc.html"), "{{ content | word_count }}").unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("wc");
        page.rendered_html = Some("one two three four five".to_string());
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains('5'), "Got: {html}");
    }

    #[test]
    fn truncate_words_filter_works() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("tw.html"),
            "{{ content | truncate_words(count=3) }}",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("tw");
        page.rendered_html = Some("one two three four five six".to_string());
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert!(html.contains("one two three..."), "Got: {html}");
        assert!(!html.contains("four"));
    }

    #[test]
    fn truncate_words_short_content_no_ellipsis() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("tw.html"),
            "{{ content | truncate_words(count=10) }}",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("tw");
        page.rendered_html = Some("just three words".to_string());
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();
        assert_eq!(html.trim(), "just three words");
        assert!(!html.contains("..."));
    }

    // --- MiniJinja tests ---

    #[test]
    fn minijinja_rendering() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("page.jinja"),
            "<html><body><h1>{{ page.title }}</h1>{{ content }}</body></html>",
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
    fn minijinja_with_assets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("withassets.jinja"),
            concat!(
                "<link rel=\"stylesheet\" href=\"{{ assets.css_path }}\">",
                "<script src=\"{{ assets.js_path }}\"></script>",
                "<div>{{ content }}</div>",
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
    fn minijinja_filters() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mjfilters.jinja"),
            concat!(
                "<p>{{ content | reading_time }}</p>",
                "<p>{{ content | word_count }}</p>",
                "<p>{{ content | truncate_words(count=2) }}</p>",
                "<p>{{ \"**bold**\" | markdownify }}</p>",
                "<p>{{ \"<b>hi</b>\" | plainify }}</p>",
                "<p>{{ \"my-slug\" | humanize }}</p>",
                "<p>{{ \"post\" | pluralize }}</p>",
                "<p>{{ \"posts\" | singularize }}</p>",
                "<p>{{ \"My Title\" | urlize }}</p>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("mjfilters");
        page.rendered_html = Some("one two three four five".to_string());
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("1 min read"), "Got: {html}");
        assert!(html.contains("<p>5</p>"), "Got: {html}");
        assert!(html.contains("one two..."), "Got: {html}");
        assert!(html.contains("<strong>bold</strong>"), "Got: {html}");
        assert!(html.contains("<p>hi</p>"), "Got: {html}");
        assert!(html.contains("My Slug"), "Got: {html}");
        assert!(html.contains("posts"), "Got: {html}");
        assert!(html.contains("post"), "Got: {html}");
        assert!(html.contains("my-title"), "Got: {html}");
    }

    #[test]
    fn mixed_engines_with_minijinja() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("default.html"),
            "<html>{{ page.title }} {{ content }}</html>",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("blog.hbs"),
            "<article>{{page.title}} {{{content}}}</article>",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("landing.jinja"),
            "<section>{{ page.title }} {{ content }}</section>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let config = test_config();

        let tera_html = engine.render(&test_page("default"), &config).unwrap();
        assert!(tera_html.contains("Test Page"));
        assert!(tera_html.contains("<html>"));

        let hbs_html = engine.render(&test_page("blog"), &config).unwrap();
        assert!(hbs_html.contains("Test Page"));
        assert!(hbs_html.contains("<article>"));

        let mj_html = engine.render(&test_page("landing"), &config).unwrap();
        assert!(mj_html.contains("Test Page"));
        assert!(mj_html.contains("<section>"));
    }

    #[test]
    fn default_engine_minijinja() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("default.html"),
            "<html>{{ page.title }} {{ content }}</html>",
        )
        .unwrap();

        let engine = TemplateEngine::new_with_default(dir.path(), "minijinja").unwrap();
        let html = engine
            .render(&test_page("default"), &test_config())
            .unwrap();
        assert!(html.contains("Test Page"));
        assert!(html.contains("<p>Hello world</p>"));
    }

    #[test]
    fn minijinja_template_inheritance() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("base.jinja"),
            concat!(
                "<!DOCTYPE html><html><head><title>{% block title %}Default{% endblock %}</title></head>",
                "<body>{% block body %}{% endblock %}</body></html>",
            ),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("child.jinja"),
            concat!(
                "{% extends \"base.jinja\" %}",
                "{% block title %}{{ page.title }}{% endblock %}",
                "{% block body %}<article>{{ content }}</article>{% endblock %}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("child");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<!DOCTYPE html>"), "Got: {html}");
        assert!(html.contains("<title>Test Page</title>"), "Got: {html}");
        assert!(
            html.contains("<article><p>Hello world</p></article>"),
            "Got: {html}"
        );
    }

    #[test]
    fn minijinja_includes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("header.jinja"),
            "<header>{{ site.title }}</header>",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("withinclude.jinja"),
            concat!(
                "{% include \"header.jinja\" %}",
                "<main>{{ content }}</main>",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("withinclude");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("<header>My Site</header>"), "Got: {html}");
        assert!(
            html.contains("<main><p>Hello world</p></main>"),
            "Got: {html}"
        );
    }

    #[test]
    fn minijinja_for_loops_and_conditionals() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("looptest.jinja"),
            concat!(
                "{% for tag in page.tags %}[{{ tag }}]{% endfor %}",
                "{% if page.date %}DATE:{{ page.date }}{% endif %}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = full_page("looptest");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("[rust]"), "Got: {html}");
        assert!(html.contains("[web]"), "Got: {html}");
        assert!(html.contains("[ssg]"), "Got: {html}");
        assert!(html.contains("DATE:2025-06-15"), "Got: {html}");
    }

    #[test]
    fn minijinja_has_math_and_katex() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mathpage.jinja"),
            concat!(
                "{% if page.has_math %}MATH_DETECTED{% endif %}",
                "<link href=\"{{ katex_css }}\">",
                "<script src=\"{{ katex_js }}\"></script>",
                "<script src=\"{{ katex_auto_render_js }}\"></script>",
                "{{ content }}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("mathpage");
        page.rendered_html = Some(
            "<p>Inline math: <span class=\"math math-inline\">x^2</span></p>".to_string(),
        );
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("MATH_DETECTED"), "Got: {html}");
        assert!(html.contains("katex.min.css"), "Got: {html}");
        assert!(html.contains("katex.min.js"), "Got: {html}");
        assert!(html.contains("auto-render.min.js"), "Got: {html}");
    }

    #[test]
    fn minijinja_missing_template_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();

        // Use default engine = minijinja so the missing layout routes through minijinja
        let engine = TemplateEngine::new_with_default(dir.path(), "minijinja").unwrap();
        let page = test_page("missing");
        let result = engine.render(&page, &config);
        assert!(result.is_err(), "Expected error for missing template");
    }

    #[test]
    fn minijinja_syntax_error_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("broken.jinja"),
            "{% if %}<p>bad syntax</p>",
        )
        .unwrap();

        let result = TemplateEngine::new(dir.path());
        assert!(
            result.is_err(),
            "Expected error for template with syntax error"
        );
    }

    #[test]
    fn minijinja_j2_extension() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("page.j2"),
            "<html><body><h1>{{ page.title }}</h1>{{ content }}</body></html>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("page");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("Test Page"), "Got: {html}");
        assert!(html.contains("<p>Hello world</p>"), "Got: {html}");
    }

    #[test]
    fn minijinja_jinja2_extension() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("page.jinja2"),
            "<html><body><h1>{{ page.title }}</h1>{{ content }}</body></html>",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("page");
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("Test Page"), "Got: {html}");
        assert!(html.contains("<p>Hello world</p>"), "Got: {html}");
    }

    #[test]
    fn minijinja_data_context() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("datactx.jinja"),
            concat!(
                "<p>Pages: {{ data.pages | length }}</p>",
                "<p>Sections: {{ data.sections | length }}</p>",
                "{% if data.paginator %}PAG{% endif %}",
                "{% if data.terms %}TERMS{% endif %}",
                "{{ content }}",
            ),
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let page = test_page("datactx");
        let config = test_config();

        let data = serde_json::json!({
            "pages": [
                {"title": "Page One", "slug": "page-one"},
                {"title": "Page Two", "slug": "page-two"},
            ],
            "sections": [
                {"name": "blog"},
            ],
            "paginator": {"current": 1, "total": 3},
            "terms": [
                {"name": "rust", "count": 5},
            ]
        });

        let html = engine
            .render_full(&page, &config, None, Some(&data))
            .unwrap();

        assert!(html.contains("<p>Pages: 2</p>"), "Got: {html}");
        assert!(html.contains("<p>Sections: 1</p>"), "Got: {html}");
        assert!(html.contains("PAG"), "Got: {html}");
        assert!(html.contains("TERMS"), "Got: {html}");
    }

    #[test]
    fn minijinja_truncate_words_filter_with_kwargs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("trunc.jinja"),
            "{{ content | truncate_words(count=3) }}",
        )
        .unwrap();

        let engine = TemplateEngine::new(dir.path()).unwrap();
        let mut page = test_page("trunc");
        page.rendered_html = Some("one two three four five six".to_string());
        let config = test_config();
        let html = engine.render(&page, &config).unwrap();

        assert!(html.contains("one two three..."), "Got: {html}");
        assert!(!html.contains("four"), "Got: {html}");
    }
}
