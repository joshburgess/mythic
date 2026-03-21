//! Internationalization (i18n) support.
//!
//! Content can be organized by locale directories (`content/en/`, `content/es/`)
//! or via `locale:` frontmatter field. Generates locale-prefixed URLs and
//! hreflang link tags.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

use crate::config::I18nConfig;
use crate::page::Page;

/// Translation data loaded from _data/i18n/{locale}.yaml files.
#[derive(Debug, Clone, Default)]
pub struct Translations {
    locales: HashMap<String, Value>,
}

impl Translations {
    /// Load translation files from the i18n data directory.
    pub fn load(data_dir: &Path, config: &I18nConfig) -> Result<Self> {
        let i18n_dir = data_dir.join("i18n");
        let mut locales = HashMap::new();

        for locale in &config.locales {
            for ext in &["yaml", "yml", "json", "toml"] {
                let path = i18n_dir.join(format!("{locale}.{ext}"));
                if path.exists() {
                    let content = std::fs::read_to_string(&path)
                        .with_context(|| format!("Failed to read: {}", path.display()))?;
                    let val: Value = match *ext {
                        "yaml" | "yml" => serde_yaml::from_str(&content)?,
                        "json" => serde_json::from_str(&content)?,
                        "toml" => {
                            let tv: toml::Value = toml::from_str(&content)?;
                            crate::data::toml_to_json_pub(tv)
                        }
                        _ => continue,
                    };
                    locales.insert(locale.clone(), val);
                    break;
                }
            }
        }

        Ok(Translations { locales })
    }

    /// Look up a translation key for a locale.
    /// Supports dotted keys: `t("nav.home")` → translations[locale]["nav"]["home"]
    pub fn translate(&self, locale: &str, key: &str) -> Option<String> {
        let data = self.locales.get(locale)?;
        let mut current = data;

        for part in key.split('.') {
            current = current.get(part)?;
        }

        current.as_str().map(String::from)
    }
}

/// A translation link for hreflang tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationLink {
    pub locale: String,
    pub url: String,
}

/// Process pages for i18n: detect locales, adjust slugs, find translations.
pub fn process_i18n(
    pages: &mut Vec<Page>,
    config: &I18nConfig,
) {
    // Detect locale from directory structure or frontmatter
    for page in pages.iter_mut() {
        let locale = detect_locale(page, config);
        page.frontmatter.locale = Some(locale);
    }

    // Adjust slugs for non-default locales
    for page in pages.iter_mut() {
        let locale = page.frontmatter.locale.as_deref().unwrap_or(&config.default_locale);
        if locale != config.default_locale {
            // Strip locale prefix from slug if it came from directory structure
            let stripped = page.slug
                .strip_prefix(&format!("{locale}/"))
                .unwrap_or(&page.slug)
                .to_string();
            page.slug = format!("{locale}/{stripped}");
        }
    }
}

fn detect_locale(page: &Page, config: &I18nConfig) -> String {
    // Check frontmatter first
    if let Some(ref locale) = page.frontmatter.locale {
        if config.locales.contains(locale) {
            return locale.clone();
        }
    }

    // Check if the slug starts with a locale directory
    for locale in &config.locales {
        if page.slug.starts_with(&format!("{locale}/")) || page.slug == *locale {
            return locale.clone();
        }
    }

    config.default_locale.clone()
}

/// Find translation links for a page (other locale versions of the same content).
pub fn find_translations(page: &Page, all_pages: &[Page], config: &I18nConfig) -> Vec<TranslationLink> {
    let current_locale = page.frontmatter.locale.as_deref().unwrap_or(&config.default_locale);

    // Get the "base" slug (without locale prefix)
    let base_slug = strip_locale_prefix(&page.slug, config);

    let mut translations = Vec::new();

    for other in all_pages {
        let other_locale = other.frontmatter.locale.as_deref().unwrap_or(&config.default_locale);
        if other_locale == current_locale {
            continue;
        }

        let other_base = strip_locale_prefix(&other.slug, config);
        if other_base == base_slug {
            translations.push(TranslationLink {
                locale: other_locale.to_string(),
                url: format!("/{}/", other.slug),
            });
        }
    }

    translations
}

fn strip_locale_prefix<'a>(slug: &'a str, config: &I18nConfig) -> &'a str {
    for locale in &config.locales {
        if let Some(stripped) = slug.strip_prefix(&format!("{locale}/")) {
            return stripped;
        }
    }
    slug
}

/// Generate hreflang link tags for a page.
pub fn generate_hreflang_tags(
    page: &Page,
    all_pages: &[Page],
    config: &I18nConfig,
    base_url: &str,
) -> String {
    let translations = find_translations(page, all_pages, config);
    let current_locale = page.frontmatter.locale.as_deref().unwrap_or(&config.default_locale);
    let base_url = base_url.trim_end_matches('/');

    let mut tags = String::new();

    // Self reference
    tags.push_str(&format!(
        "<link rel=\"alternate\" hreflang=\"{current_locale}\" href=\"{base_url}/{}/\">\n",
        page.slug
    ));

    // Other translations
    for t in &translations {
        tags.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"{}\" href=\"{base_url}{}\">\n",
            t.locale, t.url
        ));
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::path::PathBuf;

    fn page_with_locale(slug: &str, locale: Option<&str>) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: slug.to_string(),
                locale: locale.map(String::from),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    fn test_i18n_config() -> I18nConfig {
        I18nConfig {
            default_locale: "en".to_string(),
            locales: vec!["en".to_string(), "es".to_string(), "fr".to_string()],
        }
    }

    #[test]
    fn locale_url_generation() {
        let config = test_i18n_config();
        let mut pages = vec![
            page_with_locale("en/about", None),
            page_with_locale("es/about", None),
        ];

        process_i18n(&mut pages, &config);

        // Default locale (en) keeps its prefix since it came from directory
        assert_eq!(pages[0].frontmatter.locale.as_deref(), Some("en"));
        assert_eq!(pages[1].frontmatter.locale.as_deref(), Some("es"));
    }

    #[test]
    fn hreflang_tags() {
        let config = test_i18n_config();
        let pages = vec![
            page_with_locale("en/about", Some("en")),
            page_with_locale("es/about", Some("es")),
            page_with_locale("fr/about", Some("fr")),
        ];

        let tags = generate_hreflang_tags(&pages[0], &pages, &config, "https://example.com");
        assert!(tags.contains("hreflang=\"en\""));
        assert!(tags.contains("hreflang=\"es\""));
        assert!(tags.contains("hreflang=\"fr\""));
    }

    #[test]
    fn translation_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let i18n_dir = dir.path().join("i18n");
        std::fs::create_dir_all(&i18n_dir).unwrap();

        std::fs::write(
            i18n_dir.join("en.yaml"),
            "nav:\n  home: Home\n  about: About",
        ).unwrap();
        std::fs::write(
            i18n_dir.join("es.yaml"),
            "nav:\n  home: Inicio\n  about: Acerca de",
        ).unwrap();

        let config = test_i18n_config();
        let translations = Translations::load(dir.path(), &config).unwrap();

        assert_eq!(translations.translate("en", "nav.home"), Some("Home".to_string()));
        assert_eq!(translations.translate("es", "nav.home"), Some("Inicio".to_string()));
        assert_eq!(translations.translate("es", "nav.about"), Some("Acerca de".to_string()));
        assert_eq!(translations.translate("en", "missing.key"), None);
    }

    #[test]
    fn translations_list() {
        let config = test_i18n_config();
        let pages = vec![
            page_with_locale("en/about", Some("en")),
            page_with_locale("es/about", Some("es")),
            page_with_locale("en/contact", Some("en")),
        ];

        let translations = find_translations(&pages[0], &pages, &config);
        assert_eq!(translations.len(), 1);
        assert_eq!(translations[0].locale, "es");
        assert!(translations[0].url.contains("es/about"));
    }

    #[test]
    fn frontmatter_locale_detection() {
        let config = test_i18n_config();
        let mut pages = vec![
            page_with_locale("special-page", Some("es")),
        ];

        process_i18n(&mut pages, &config);
        assert_eq!(pages[0].frontmatter.locale.as_deref(), Some("es"));
    }

    #[test]
    fn missing_translation_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let i18n_dir = dir.path().join("i18n");
        std::fs::create_dir_all(&i18n_dir).unwrap();
        std::fs::write(i18n_dir.join("en.yaml"), "greeting: Hello").unwrap();

        let config = test_i18n_config();
        let translations = Translations::load(dir.path(), &config).unwrap();

        // Missing key
        assert_eq!(translations.translate("en", "nonexistent"), None);
        // Missing locale
        assert_eq!(translations.translate("de", "greeting"), None);
        // Missing nested key
        assert_eq!(translations.translate("en", "deeply.nested.missing"), None);
    }

    #[test]
    fn default_locale_pages_keep_original_slug() {
        let config = test_i18n_config();
        let mut pages = vec![
            page_with_locale("about", Some("en")),
        ];

        process_i18n(&mut pages, &config);

        // Default locale (en) should keep its slug as-is
        assert_eq!(pages[0].slug, "about");
    }

    #[test]
    fn pages_without_locale_get_default() {
        let config = test_i18n_config();
        let mut pages = vec![
            page_with_locale("contact", None),
        ];

        process_i18n(&mut pages, &config);

        // Should be assigned the default locale "en"
        assert_eq!(pages[0].frontmatter.locale.as_deref(), Some("en"));
    }

    #[test]
    fn empty_translations_object() {
        let dir = tempfile::tempdir().unwrap();
        // No i18n directory at all
        let config = test_i18n_config();
        let translations = Translations::load(dir.path(), &config).unwrap();

        // Should succeed but return None for everything
        assert_eq!(translations.translate("en", "anything"), None);
        assert_eq!(translations.translate("es", "anything"), None);
        assert!(translations.locales.is_empty());
    }
}
