//! Content linting system for build-time quality checks.
//!
//! Provides configurable rules that run during the build pipeline to warn about
//! content quality issues: missing fields, word count violations, long paragraphs,
//! empty titles, and orphan pages.

use crate::page::Page;
use serde::{Deserialize, Serialize};

/// Configuration for the content linting system.
///
/// All rules are opt-in except `enabled`, which defaults to `true`.
/// A zero value for numeric thresholds means that rule is disabled.
///
/// # Example (TOML)
///
/// ```toml
/// [lint]
/// min_word_count = 100
/// require_tags = true
/// required_fields = ["title", "date"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Minimum word count per page (0 = disabled).
    #[serde(default)]
    pub min_word_count: usize,
    /// Maximum word count per page (0 = disabled).
    #[serde(default)]
    pub max_word_count: usize,
    /// Required frontmatter fields (e.g., `["title", "date"]`).
    #[serde(default)]
    pub required_fields: Vec<String>,
    /// Warn on pages with no tags.
    #[serde(default)]
    pub require_tags: bool,
    /// Warn on pages with no date.
    #[serde(default)]
    pub require_date: bool,
    /// Maximum heading level allowed at start (e.g., 2 means content shouldn't start with h1).
    #[serde(default)]
    pub max_start_heading: u32,
}

fn default_true() -> bool {
    true
}

/// A single lint warning produced during content validation.
#[derive(Debug)]
pub struct LintWarning {
    /// Slug of the page that triggered the warning.
    pub slug: String,
    /// Machine-readable rule identifier (e.g., `"min-word-count"`).
    pub rule: String,
    /// Human-readable description of the issue.
    pub message: String,
}

/// Run all configured lint rules against the given pages.
///
/// Returns an empty list when linting is disabled or all pages pass.
pub fn lint_pages(pages: &[Page], config: &LintConfig) -> Vec<LintWarning> {
    if !config.enabled {
        return Vec::new();
    }

    let mut warnings = Vec::new();

    for page in pages {
        // Skip generated/virtual pages
        if page.source_path.to_string_lossy().starts_with('<') {
            continue;
        }

        let word_count = page.raw_content.split_whitespace().count();

        // Min word count
        if config.min_word_count > 0 && word_count < config.min_word_count {
            warnings.push(LintWarning {
                slug: page.slug.clone(),
                rule: "min-word-count".to_string(),
                message: format!(
                    "Page has {} words, minimum is {}",
                    word_count, config.min_word_count
                ),
            });
        }

        // Max word count
        if config.max_word_count > 0 && word_count > config.max_word_count {
            warnings.push(LintWarning {
                slug: page.slug.clone(),
                rule: "max-word-count".to_string(),
                message: format!(
                    "Page has {} words, maximum is {}",
                    word_count, config.max_word_count
                ),
            });
        }

        // Required tags
        if config.require_tags {
            let has_tags = page
                .frontmatter
                .tags
                .as_ref()
                .map(|t| !t.is_empty())
                .unwrap_or(false);
            if !has_tags {
                warnings.push(LintWarning {
                    slug: page.slug.clone(),
                    rule: "require-tags".to_string(),
                    message: "Page has no tags".to_string(),
                });
            }
        }

        // Required date
        if config.require_date && page.frontmatter.date.is_none() {
            warnings.push(LintWarning {
                slug: page.slug.clone(),
                rule: "require-date".to_string(),
                message: "Page has no date".to_string(),
            });
        }

        // Required fields (check extra)
        for field in &config.required_fields {
            let has_field = match field.as_str() {
                "title" => !page.frontmatter.title.is_empty(),
                "date" => page.frontmatter.date.is_some(),
                "tags" => page
                    .frontmatter
                    .tags
                    .as_ref()
                    .map(|t| !t.is_empty())
                    .unwrap_or(false),
                "layout" => page.frontmatter.layout.is_some(),
                _ => page
                    .frontmatter
                    .extra
                    .as_ref()
                    .map(|e| e.contains_key(field))
                    .unwrap_or(false),
            };
            if !has_field {
                warnings.push(LintWarning {
                    slug: page.slug.clone(),
                    rule: "required-field".to_string(),
                    message: format!("Missing required field: {field}"),
                });
            }
        }

        // Readability: detect very long paragraphs (>300 words without a break)
        for (i, paragraph) in page.raw_content.split("\n\n").enumerate() {
            let p_words = paragraph.split_whitespace().count();
            if p_words > 300 {
                warnings.push(LintWarning {
                    slug: page.slug.clone(),
                    rule: "long-paragraph".to_string(),
                    message: format!(
                        "Paragraph {} has {} words (consider breaking it up)",
                        i + 1,
                        p_words
                    ),
                });
            }
        }

        // Detect empty title
        if page.frontmatter.title.is_empty() {
            warnings.push(LintWarning {
                slug: page.slug.clone(),
                rule: "empty-title".to_string(),
                message: "Page has an empty title".to_string(),
            });
        }
    }

    warnings
}

/// Find pages that are not linked from any other page.
///
/// Scans rendered HTML for internal `href="/slug/"` patterns and returns
/// slugs of pages that are never referenced. The `"index"` page is always
/// considered linked.
pub fn find_orphan_pages(pages: &[Page]) -> Vec<String> {
    use std::collections::HashSet;

    let all_slugs: HashSet<&str> = pages.iter().map(|p| p.slug.as_str()).collect();
    let mut linked: HashSet<String> = HashSet::new();

    // Always consider index as linked
    linked.insert("index".to_string());

    // Scan all rendered HTML for internal links
    for page in pages {
        if let Some(ref html) = page.rendered_html {
            // Find href="/slug/" patterns
            let mut remaining = html.as_str();
            while let Some(start) = remaining.find("href=\"/") {
                let after = &remaining[start + 7..]; // skip past href="/
                if let Some(end) = after.find('"') {
                    let path = after[..end].trim_end_matches('/');
                    if !path.is_empty() {
                        linked.insert(path.to_string());
                    }
                }
                remaining = &remaining[start + 7..];
            }
        }
    }

    // Find pages that exist but are never linked to
    let mut orphans: Vec<String> = all_slugs
        .iter()
        .filter(|slug| !linked.contains(**slug))
        .map(|s| s.to_string())
        .collect();
    orphans.sort();
    orphans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use compact_str::CompactString;
    use std::path::PathBuf;

    fn make_page(slug: &str, title: &str, content: &str) -> Page {
        Page {
            source_path: PathBuf::from(format!("content/{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: CompactString::new(title),
                ..Default::default()
            },
            raw_content: content.to_string(),
            rendered_html: None,
            body_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    fn default_config() -> LintConfig {
        LintConfig {
            enabled: true,
            min_word_count: 0,
            max_word_count: 0,
            required_fields: Vec::new(),
            require_tags: false,
            require_date: false,
            max_start_heading: 0,
        }
    }

    #[test]
    fn min_word_count_warning() {
        let mut config = default_config();
        config.min_word_count = 10;
        let pages = vec![make_page("short", "Short", "only three words")];
        let warnings = lint_pages(&pages, &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "min-word-count");
        assert!(warnings[0].message.contains("3 words"));
    }

    #[test]
    fn max_word_count_warning() {
        let mut config = default_config();
        config.max_word_count = 5;
        let content = "one two three four five six seven";
        let pages = vec![make_page("long", "Long", content)];
        let warnings = lint_pages(&pages, &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "max-word-count");
        assert!(warnings[0].message.contains("7 words"));
    }

    #[test]
    fn require_tags_warning() {
        let mut config = default_config();
        config.require_tags = true;
        let pages = vec![make_page("notags", "No Tags", "some content here")];
        let warnings = lint_pages(&pages, &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "require-tags");
    }

    #[test]
    fn require_date_warning() {
        let mut config = default_config();
        config.require_date = true;
        let pages = vec![make_page("nodate", "No Date", "some content here")];
        let warnings = lint_pages(&pages, &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "require-date");
    }

    #[test]
    fn required_fields_checks_extra() {
        let mut config = default_config();
        config.required_fields = vec!["author".to_string()];
        let pages = vec![make_page("noextra", "Title", "some content here")];
        let warnings = lint_pages(&pages, &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "required-field");
        assert!(warnings[0].message.contains("author"));
    }

    #[test]
    fn empty_title_detected() {
        let config = default_config();
        let pages = vec![make_page("empty", "", "some content here")];
        let warnings = lint_pages(&pages, &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "empty-title");
    }

    #[test]
    fn long_paragraph_detected() {
        let config = default_config();
        // Create a paragraph with more than 300 words
        let long_para: String = (0..310)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let pages = vec![make_page("longpara", "Title", &long_para)];
        let warnings = lint_pages(&pages, &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "long-paragraph");
        assert!(warnings[0].message.contains("310 words"));
    }

    #[test]
    fn disabled_config_returns_no_warnings() {
        let mut config = default_config();
        config.enabled = false;
        config.min_word_count = 1000;
        config.require_tags = true;
        config.require_date = true;
        let pages = vec![make_page("test", "Test", "short")];
        let warnings = lint_pages(&pages, &config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn orphan_page_detection() {
        let mut home = make_page("index", "Home", "welcome");
        home.rendered_html = Some("<a href=\"/about/\">About</a>".to_string());

        let mut about = make_page("about", "About", "about page");
        about.rendered_html = Some("<a href=\"/\">Home</a>".to_string());

        let orphan = make_page("orphan", "Orphan", "nobody links here");

        let pages = vec![home, about, orphan];
        let orphans = find_orphan_pages(&pages);
        assert_eq!(orphans, vec!["orphan"]);
    }
}
