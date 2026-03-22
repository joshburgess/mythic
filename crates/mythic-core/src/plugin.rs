//! Hook-based plugin system for extending the build pipeline.
//!
//! Implement the [`Plugin`] trait to create native Rust plugins, or use
//! [Rhai scripts](crate::rhai_plugin) for user-defined plugins without compilation.
//!
//! # Hook points
//!
//! | Hook | When | Can mutate |
//! |------|------|------------|
//! | `on_pre_build` | Before anything runs | Config (read-only) |
//! | `on_page_discovered` | After frontmatter parsed | Page |
//! | `on_pre_render` | Before markdown rendering | Page |
//! | `on_post_render` | After markdown rendering | Page |
//! | `on_post_build` | After all pages written | Report (read-only) |
//!
//! # Example
//!
//! ```rust,no_run
//! use mythic_core::plugin::Plugin;
//! use mythic_core::page::Page;
//! use anyhow::Result;
//!
//! struct WordCountPlugin;
//!
//! impl Plugin for WordCountPlugin {
//!     fn name(&self) -> &str { "word-count" }
//!
//!     fn on_page_discovered(&self, page: &mut Page) -> Result<()> {
//!         let count = page.raw_content.split_whitespace().count();
//!         let extra = page.frontmatter.extra.get_or_insert_with(Default::default);
//!         extra.insert("word_count".into(), count.into());
//!         Ok(())
//!     }
//! }
//! ```

use anyhow::Result;

use crate::build::BuildReport;
use crate::config::SiteConfig;
use crate::page::Page;

/// Trait for build pipeline plugins.
///
/// All hooks have default no-op implementations. Plugins only need to
/// override the hooks they care about.
pub trait Plugin: Send + Sync {
    /// Human-readable plugin name.
    fn name(&self) -> &str;

    /// Called before the build starts.
    fn on_pre_build(&self, _config: &SiteConfig) -> Result<()> {
        Ok(())
    }

    /// Called after a page is discovered (frontmatter parsed, before rendering).
    fn on_page_discovered(&self, _page: &mut Page) -> Result<()> {
        Ok(())
    }

    /// Called just before a page's markdown is rendered.
    fn on_pre_render(&self, _page: &mut Page) -> Result<()> {
        Ok(())
    }

    /// Called after a page's markdown has been rendered to HTML.
    fn on_post_render(&self, _page: &mut Page) -> Result<()> {
        Ok(())
    }

    /// Called after the entire build completes.
    fn on_post_build(&self, _report: &BuildReport) -> Result<()> {
        Ok(())
    }
}

/// Manages a collection of plugins and dispatches hooks.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Run on_pre_build for all plugins in registration order.
    pub fn run_pre_build(&self, config: &SiteConfig) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_pre_build(config).map_err(|e| {
                anyhow::anyhow!("Plugin '{}' failed on_pre_build: {e}", plugin.name())
            })?;
        }
        Ok(())
    }

    /// Run on_page_discovered for all plugins on a single page.
    pub fn run_page_discovered(&self, page: &mut Page) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_page_discovered(page).map_err(|e| {
                anyhow::anyhow!(
                    "Plugin '{}' failed on_page_discovered for '{}': {e}",
                    plugin.name(),
                    page.slug
                )
            })?;
        }
        Ok(())
    }

    /// Run on_pre_render for all plugins on a single page.
    pub fn run_pre_render(&self, page: &mut Page) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_pre_render(page).map_err(|e| {
                anyhow::anyhow!(
                    "Plugin '{}' failed on_pre_render for '{}': {e}",
                    plugin.name(),
                    page.slug
                )
            })?;
        }
        Ok(())
    }

    /// Run on_post_render for all plugins on a single page.
    pub fn run_post_render(&self, page: &mut Page) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_post_render(page).map_err(|e| {
                anyhow::anyhow!(
                    "Plugin '{}' failed on_post_render for '{}': {e}",
                    plugin.name(),
                    page.slug
                )
            })?;
        }
        Ok(())
    }

    /// Run on_post_build for all plugins.
    pub fn run_post_build(&self, report: &BuildReport) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_post_build(report).map_err(|e| {
                anyhow::anyhow!("Plugin '{}' failed on_post_build: {e}", plugin.name())
            })?;
        }
        Ok(())
    }

    /// Run on_page_discovered for all pages.
    pub fn run_all_discovered(&self, pages: &mut [Page]) -> Result<()> {
        for page in pages.iter_mut() {
            self.run_page_discovered(page)?;
        }
        Ok(())
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// --- Built-in plugins ---

/// Estimates reading time and adds it to page.extra["reading_time"].
pub struct ReadingTimePlugin {
    words_per_minute: usize,
}

impl ReadingTimePlugin {
    pub fn new() -> Self {
        ReadingTimePlugin {
            words_per_minute: 200,
        }
    }
}

impl Default for ReadingTimePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ReadingTimePlugin {
    fn name(&self) -> &str {
        "reading-time"
    }

    fn on_page_discovered(&self, page: &mut Page) -> Result<()> {
        let word_count = page.raw_content.split_whitespace().count();
        let minutes = (word_count + self.words_per_minute - 1) / self.words_per_minute;
        let reading_time = if minutes <= 1 {
            "1 min read".to_string()
        } else {
            format!("{minutes} min read")
        };

        let extra = page
            .frontmatter
            .extra
            .get_or_insert_with(std::collections::HashMap::new);
        extra.insert(
            "reading_time".to_string(),
            serde_json::Value::String(reading_time),
        );
        extra.insert(
            "word_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(word_count)),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn test_page(slug: &str, content: &str) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: "Test".into(),
                ..Default::default()
            },
            raw_content: content.to_string(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    struct OrderTracker {
        log: Arc<Mutex<Vec<String>>>,
        id: String,
    }

    impl Plugin for OrderTracker {
        fn name(&self) -> &str {
            &self.id
        }

        fn on_page_discovered(&self, _page: &mut Page) -> Result<()> {
            self.log.lock().unwrap().push(self.id.clone());
            Ok(())
        }
    }

    #[test]
    fn hooks_execute_in_registration_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut manager = PluginManager::new();

        manager.register(Box::new(OrderTracker {
            log: log.clone(),
            id: "first".to_string(),
        }));
        manager.register(Box::new(OrderTracker {
            log: log.clone(),
            id: "second".to_string(),
        }));
        manager.register(Box::new(OrderTracker {
            log: log.clone(),
            id: "third".to_string(),
        }));

        let mut page = test_page("test", "content");
        manager.run_page_discovered(&mut page).unwrap();

        let entries = log.lock().unwrap();
        assert_eq!(*entries, vec!["first", "second", "third"]);
    }

    struct MutatingPlugin;

    impl Plugin for MutatingPlugin {
        fn name(&self) -> &str {
            "mutator"
        }

        fn on_page_discovered(&self, page: &mut Page) -> Result<()> {
            page.frontmatter.title = "Mutated".into();
            Ok(())
        }
    }

    #[test]
    fn plugins_can_mutate_pages() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(MutatingPlugin));

        let mut page = test_page("test", "content");
        assert_eq!(page.frontmatter.title, "Test");

        manager.run_page_discovered(&mut page).unwrap();
        assert_eq!(page.frontmatter.title, "Mutated");
    }

    struct FailingPlugin;

    impl Plugin for FailingPlugin {
        fn name(&self) -> &str {
            "failing"
        }

        fn on_page_discovered(&self, _page: &mut Page) -> Result<()> {
            anyhow::bail!("Something went wrong")
        }
    }

    #[test]
    fn error_propagation() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(FailingPlugin));

        let mut page = test_page("test", "content");
        let result = manager.run_page_discovered(&mut page);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failing"));
        assert!(err.contains("Something went wrong"));
    }

    #[test]
    fn reading_time_calculation() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(ReadingTimePlugin::new()));

        // ~400 words → 2 min read
        let words: String = (0..400).map(|_| "word ").collect();
        let mut page = test_page("test", &words);
        manager.run_page_discovered(&mut page).unwrap();

        let extra = page.frontmatter.extra.unwrap();
        assert_eq!(extra["reading_time"], "2 min read");
        assert_eq!(extra["word_count"], 400);
    }

    #[test]
    fn reading_time_short_content() {
        let mut manager = PluginManager::new();
        manager.register(Box::new(ReadingTimePlugin::new()));

        let mut page = test_page("test", "short content");
        manager.run_page_discovered(&mut page).unwrap();

        let extra = page.frontmatter.extra.unwrap();
        assert_eq!(extra["reading_time"], "1 min read");
    }
}
