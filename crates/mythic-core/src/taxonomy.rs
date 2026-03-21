//! Taxonomy system for tags, categories, and custom taxonomies.

use crate::config::{SiteConfig, TaxonomyConfig};
use crate::page::{Frontmatter, Page};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A taxonomy term with its associated pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyTerm {
    pub name: String,
    pub slug: String,
    pub pages: Vec<TaxonomyPageRef>,
}

/// Lightweight page reference for taxonomy contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyPageRef {
    pub title: String,
    pub slug: String,
    pub date: Option<String>,
    pub url: String,
}

/// A complete taxonomy with all its terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Taxonomy {
    pub config: TaxonomyConfig,
    pub terms: Vec<TaxonomyTerm>,
}

/// Extract taxonomies from pages and generate taxonomy/term pages.
pub fn build_taxonomies(
    config: &SiteConfig,
    pages: &[Page],
) -> Vec<Taxonomy> {
    config
        .taxonomies
        .iter()
        .map(|tc| build_one_taxonomy(tc, pages))
        .collect()
}

fn build_one_taxonomy(tc: &TaxonomyConfig, pages: &[Page]) -> Taxonomy {
    let mut terms_map: HashMap<String, Vec<TaxonomyPageRef>> = HashMap::new();

    for page in pages {
        let values = extract_taxonomy_values(&page.frontmatter, &tc.name);
        for value in values {
            // Skip empty tag values
            if value.trim().is_empty() {
                continue;
            }
            let slug = slugify(&value);
            if slug.is_empty() {
                continue;
            }
            terms_map
                .entry(slug)
                .or_default()
                .push(TaxonomyPageRef {
                    title: page.frontmatter.title.clone(),
                    slug: page.slug.clone(),
                    date: page.frontmatter.date.clone(),
                    url: format!("/{}/", page.slug),
                });
        }
    }

    let mut terms: Vec<TaxonomyTerm> = terms_map
        .into_iter()
        .map(|(slug, mut pages)| {
            // Sort by date descending
            pages.sort_by(|a, b| b.date.cmp(&a.date));
            let name = slug.clone(); // Will be the slugified form
            TaxonomyTerm { name, slug, pages }
        })
        .collect();

    terms.sort_by(|a, b| a.name.cmp(&b.name));

    Taxonomy {
        config: tc.clone(),
        terms,
    }
}

fn extract_taxonomy_values(fm: &Frontmatter, taxonomy_name: &str) -> Vec<String> {
    // Check built-in "tags" field
    if taxonomy_name == "tags" {
        if let Some(ref tags) = fm.tags {
            return tags.clone();
        }
    }

    // Check extra fields
    if let Some(ref extra) = fm.extra {
        if let Some(val) = extra.get(taxonomy_name) {
            if let Some(arr) = val.as_array() {
                return arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
            if let Some(s) = val.as_str() {
                return vec![s.to_string()];
            }
        }
    }

    Vec::new()
}

fn slugify(s: &str) -> String {
    // Transliterate common programming symbols before slugifying
    let expanded = s
        .replace("++", "-plus-plus")
        .replace('+', "-plus")
        .replace('#', "-sharp")
        .replace('&', "-and")
        .replace('@', "-at");

    expanded
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Generate virtual pages for taxonomy listings and term pages.
pub fn generate_taxonomy_pages(taxonomies: &[Taxonomy]) -> Vec<Page> {
    let mut pages = Vec::new();

    for taxonomy in taxonomies {
        // Listing page: /{slug}/
        let listing_slug = taxonomy.config.slug.clone();
        pages.push(Page {
            source_path: PathBuf::from(format!("<taxonomy:{}>", taxonomy.config.name)),
            slug: listing_slug,
            frontmatter: Frontmatter {
                title: taxonomy.config.name.clone(),
                layout: Some("taxonomy_list".to_string()),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        });

        // Term pages: /{slug}/{term}/
        for term in &taxonomy.terms {
            let term_slug = format!("{}/{}", taxonomy.config.slug, term.slug);
            pages.push(Page {
                source_path: PathBuf::from(format!(
                    "<taxonomy:{}:{}>",
                    taxonomy.config.name, term.name
                )),
                slug: term_slug,
                frontmatter: Frontmatter {
                    title: term.name.clone(),
                    layout: Some("taxonomy_term".to_string()),
                    ..Default::default()
                },
                raw_content: String::new(),
                rendered_html: None,
                output_path: None,
                content_hash: 0,
            toc: Vec::new(),
            });
        }
    }

    pages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SiteConfig, TaxonomyConfig};

    fn page_with_tags(title: &str, slug: &str, tags: Vec<&str>, date: Option<&str>) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: title.to_string(),
                tags: Some(tags.into_iter().map(String::from).collect()),
                date: date.map(String::from),
                ..Default::default()
            },
            raw_content: String::new(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }
    }

    fn config_with_tags() -> SiteConfig {
        let mut config = SiteConfig::for_testing("Test", "http://localhost");
        config.taxonomies.push(TaxonomyConfig {
            name: "tags".to_string(),
            slug: "tags".to_string(),
            feed: true,
        });
        config
    }

    #[test]
    fn extracts_tags_and_builds_terms() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post A", "a", vec!["rust", "web"], Some("2024-02-01")),
            page_with_tags("Post B", "b", vec!["rust"], Some("2024-01-15")),
            page_with_tags("Post C", "c", vec!["web"], Some("2024-03-01")),
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        assert_eq!(taxonomies.len(), 1);

        let tags = &taxonomies[0];
        assert_eq!(tags.terms.len(), 2); // "rust" and "web"

        let rust_term = tags.terms.iter().find(|t| t.name == "rust").unwrap();
        assert_eq!(rust_term.pages.len(), 2);
        // Sorted by date descending
        assert_eq!(rust_term.pages[0].title, "Post A");
    }

    #[test]
    fn generates_listing_and_term_pages() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "post", vec!["rust"], None),
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        let generated = generate_taxonomy_pages(&taxonomies);

        // Should have: 1 listing page + 1 term page
        assert_eq!(generated.len(), 2);
        assert!(generated.iter().any(|p| p.slug == "tags"));
        assert!(generated.iter().any(|p| p.slug == "tags/rust"));
    }

    #[test]
    fn multiple_taxonomies_independent() {
        let mut config = SiteConfig::for_testing("Test", "http://localhost");
        config.taxonomies.push(TaxonomyConfig {
            name: "tags".to_string(),
            slug: "tags".to_string(),
            feed: true,
        });
        config.taxonomies.push(TaxonomyConfig {
            name: "categories".to_string(),
            slug: "category".to_string(),
            feed: false,
        });

        let mut pages = vec![page_with_tags("Post", "post", vec!["rust"], None)];
        // Add category via extra
        let extra = pages[0].frontmatter.extra.get_or_insert_with(HashMap::new);
        extra.insert(
            "categories".to_string(),
            serde_json::json!(["tutorials"]),
        );

        let taxonomies = build_taxonomies(&config, &pages);
        assert_eq!(taxonomies.len(), 2);
        assert_eq!(taxonomies[0].terms.len(), 1); // tags: rust
        assert_eq!(taxonomies[1].terms.len(), 1); // categories: tutorials
    }

    #[test]
    fn pages_sorted_by_date_descending() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Old", "old", vec!["rust"], Some("2023-01-01")),
            page_with_tags("Mid", "mid", vec!["rust"], Some("2024-06-15")),
            page_with_tags("New", "new", vec!["rust"], Some("2024-12-01")),
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        let rust = taxonomies[0].terms.iter().find(|t| t.name == "rust").unwrap();
        assert_eq!(rust.pages[0].title, "New");
        assert_eq!(rust.pages[1].title, "Mid");
        assert_eq!(rust.pages[2].title, "Old");
    }

    #[test]
    fn no_tags_produces_empty_taxonomy() {
        let config = config_with_tags();
        let pages = vec![
            Page {
                source_path: PathBuf::from("no-tags.md"),
                slug: "no-tags".to_string(),
                frontmatter: Frontmatter {
                    title: "No Tags".to_string(),
                    ..Default::default()
                },
                raw_content: String::new(),
                rendered_html: None,
                output_path: None,
                content_hash: 0,
                toc: Vec::new(),
            },
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        assert_eq!(taxonomies[0].terms.len(), 0);
    }

    #[test]
    fn no_taxonomies_configured() {
        let config = SiteConfig::for_testing("Test", "http://localhost");
        let pages = vec![page_with_tags("Post", "post", vec!["rust"], None)];

        let taxonomies = build_taxonomies(&config, &pages);
        assert!(taxonomies.is_empty());
    }

    #[test]
    fn term_slugs_are_lowercased() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "post", vec!["Rust", "WEB"], None),
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        let terms: Vec<&str> = taxonomies[0].terms.iter().map(|t| t.slug.as_str()).collect();
        assert!(terms.contains(&"rust"));
        assert!(terms.contains(&"web"));
    }

    #[test]
    fn taxonomy_page_urls_are_correct() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "blog/my-post", vec!["rust"], None),
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        let rust = &taxonomies[0].terms[0];
        assert_eq!(rust.pages[0].url, "/blog/my-post/");
    }

    #[test]
    fn generated_pages_have_correct_layouts() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "post", vec!["rust"], None),
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        let generated = generate_taxonomy_pages(&taxonomies);

        let listing = generated.iter().find(|p| p.slug == "tags").unwrap();
        assert_eq!(listing.frontmatter.layout.as_deref(), Some("taxonomy_list"));

        let term = generated.iter().find(|p| p.slug == "tags/rust").unwrap();
        assert_eq!(term.frontmatter.layout.as_deref(), Some("taxonomy_term"));
    }

    #[test]
    fn page_in_multiple_terms() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Multi", "multi", vec!["rust", "web", "perf"], None),
        ];

        let taxonomies = build_taxonomies(&config, &pages);
        assert_eq!(taxonomies[0].terms.len(), 3);
        for term in &taxonomies[0].terms {
            assert_eq!(term.pages.len(), 1);
            assert_eq!(term.pages[0].title, "Multi");
        }
    }

    #[test]
    fn duplicate_tags_deduplicated() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "post", vec!["rust", "rust", "Rust"], None),
        ];
        let taxonomies = build_taxonomies(&config, &pages);
        // "rust" and "Rust" should collapse to the same term via slugification
        let term_slugs: Vec<&str> = taxonomies[0].terms.iter().map(|t| t.slug.as_str()).collect();
        // Should have exactly one "rust" term
        assert_eq!(term_slugs.iter().filter(|&&s| s == "rust").count(), 1);
    }

    #[test]
    fn tag_with_special_characters() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "post", vec!["C++", "C#", "node.js"], None),
        ];
        let taxonomies = build_taxonomies(&config, &pages);
        // C++ → c-plus-plus, C# → c-sharp, node.js → node-js — all distinct
        assert_eq!(taxonomies[0].terms.len(), 3,
            "C++, C#, and node.js should produce 3 distinct slugs, got: {:?}",
            taxonomies[0].terms.iter().map(|t| &t.slug).collect::<Vec<_>>());
        for term in &taxonomies[0].terms {
            assert!(!term.slug.is_empty(), "Tag '{}' produced empty slug", term.name);
        }
    }

    #[test]
    fn empty_tag_string_filtered() {
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "post", vec!["rust", "", "web"], None),
        ];
        let taxonomies = build_taxonomies(&config, &pages);
        // Empty tags should be filtered out, leaving only "rust" and "web"
        assert_eq!(taxonomies[0].terms.len(), 2);
    }

    // --- Hugo regression tests ---

    #[test]
    fn taxonomy_term_with_slash() {
        // Hugo #4090: tags like "AC/DC" or "client/server" must not break paths
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("Post", "post", vec!["AC/DC"], None),
        ];
        let taxonomies = build_taxonomies(&config, &pages);
        assert_eq!(taxonomies[0].terms.len(), 1);
        // Slug should not contain raw slash
        assert!(!taxonomies[0].terms[0].slug.contains('/'), "Slash in tag should be slugified");
    }

    #[test]
    fn taxonomy_term_with_apostrophe() {
        // Hugo #12606: apostrophes in tags should not cause duplicates
        let config = config_with_tags();
        let pages = vec![
            page_with_tags("A", "a", vec!["Disney's Resort"], None),
            page_with_tags("B", "b", vec!["Disney's Resort"], None),
        ];
        let taxonomies = build_taxonomies(&config, &pages);
        // Both pages should map to the same term
        assert_eq!(taxonomies[0].terms.len(), 1);
        assert_eq!(taxonomies[0].terms[0].pages.len(), 2);
    }
}
