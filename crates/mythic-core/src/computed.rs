//! Computed frontmatter fields via Rhai expressions.
//!
//! Allows frontmatter `extra` fields to contain Rhai expressions
//! that are evaluated at build time with access to page data.
//!
//! Convention: values starting with `rhai:` are treated as expressions.
//!
//! Example frontmatter:
//! ```yaml
//! extra:
//!   reading_time: "rhai: word_count / 200"
//!   is_long: "rhai: word_count > 1000"
//!   slug_upper: "rhai: slug.to_upper()"
//! ```

use crate::page::Page;

/// Evaluate computed frontmatter fields for all pages.
///
/// Scans `page.extra` for string values starting with `rhai:`,
/// evaluates the expression with page context, and replaces the
/// value with the result.
pub fn evaluate_computed_fields(pages: &mut [Page]) {
    let engine = rhai::Engine::new();

    for page in pages.iter_mut() {
        let extra = match page.frontmatter.extra.as_mut() {
            Some(e) => e,
            None => continue,
        };

        let word_count = page.raw_content.split_whitespace().count() as i64;
        let slug = page.slug.clone();
        let title = page.frontmatter.title.to_string();

        // Collect keys that need evaluation (can't mutate while iterating)
        let computed_keys: Vec<(String, String)> = extra
            .iter()
            .filter_map(|(k, v)| {
                v.as_str().and_then(|s| {
                    s.strip_prefix("rhai:")
                        .map(|expr| (k.clone(), expr.trim().to_string()))
                })
            })
            .collect();

        for (key, expr) in computed_keys {
            let mut scope = rhai::Scope::new();
            scope.push("word_count", word_count);
            scope.push("slug", slug.clone());
            scope.push("title", title.clone());
            scope.push("has_date", page.frontmatter.date.is_some());

            if let Some(ref date) = page.frontmatter.date {
                scope.push("date", date.to_string());
            }

            match engine.eval_with_scope::<rhai::Dynamic>(&mut scope, &expr) {
                Ok(result) => {
                    let value = dynamic_to_json(&result);
                    extra.insert(key, value);
                }
                Err(e) => {
                    eprintln!(
                        "  Warning: computed field '{}' in '{}' failed: {e}",
                        key, page.slug
                    );
                }
            }
        }
    }
}

fn dynamic_to_json(val: &rhai::Dynamic) -> serde_json::Value {
    if val.is_unit() {
        serde_json::Value::Null
    } else if let Ok(b) = val.as_bool() {
        serde_json::Value::Bool(b)
    } else if let Ok(i) = val.as_int() {
        serde_json::Value::Number(serde_json::Number::from(i))
    } else if let Ok(f) = val.as_float() {
        serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    } else if let Ok(s) = val.clone().into_string() {
        serde_json::Value::String(s)
    } else {
        serde_json::Value::String(val.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Frontmatter, Page};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn page_with_extra(
        slug: &str,
        content: &str,
        extra: HashMap<String, serde_json::Value>,
    ) -> Page {
        Page {
            source_path: PathBuf::from(format!("{slug}.md")),
            slug: slug.to_string(),
            frontmatter: Frontmatter {
                title: "Test Post".into(),
                date: Some("2024-06-15".into()),
                extra: Some(extra),
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
    fn computed_word_count() {
        let mut extra = HashMap::new();
        extra.insert(
            "wc".to_string(),
            serde_json::Value::String("rhai: word_count".to_string()),
        );
        let mut pages = vec![page_with_extra("post", "one two three four five", extra)];

        evaluate_computed_fields(&mut pages);

        let result = &pages[0].frontmatter.extra.as_ref().unwrap()["wc"];
        assert_eq!(result, &serde_json::json!(5));
    }

    #[test]
    fn computed_boolean_expression() {
        let mut extra = HashMap::new();
        extra.insert(
            "is_long".to_string(),
            serde_json::Value::String("rhai: word_count > 100".to_string()),
        );
        let content = vec!["word"; 50].join(" ");
        let mut pages = vec![page_with_extra("post", &content, extra)];

        evaluate_computed_fields(&mut pages);

        let result = &pages[0].frontmatter.extra.as_ref().unwrap()["is_long"];
        assert_eq!(result, &serde_json::json!(false));
    }

    #[test]
    fn computed_string_expression() {
        let mut extra = HashMap::new();
        extra.insert(
            "upper_slug".to_string(),
            serde_json::Value::String("rhai: slug.to_upper()".to_string()),
        );
        let mut pages = vec![page_with_extra("my-post", "content", extra)];

        evaluate_computed_fields(&mut pages);

        let result = &pages[0].frontmatter.extra.as_ref().unwrap()["upper_slug"];
        assert_eq!(result, &serde_json::json!("MY-POST"));
    }

    #[test]
    fn computed_math_expression() {
        let mut extra = HashMap::new();
        extra.insert(
            "reading_time".to_string(),
            serde_json::Value::String("rhai: word_count / 200".to_string()),
        );
        let content = vec!["word"; 600].join(" ");
        let mut pages = vec![page_with_extra("post", &content, extra)];

        evaluate_computed_fields(&mut pages);

        let result = &pages[0].frontmatter.extra.as_ref().unwrap()["reading_time"];
        assert_eq!(result, &serde_json::json!(3));
    }

    #[test]
    fn non_computed_fields_unchanged() {
        let mut extra = HashMap::new();
        extra.insert(
            "author".to_string(),
            serde_json::Value::String("Alice".to_string()),
        );
        extra.insert(
            "computed".to_string(),
            serde_json::Value::String("rhai: word_count".to_string()),
        );
        let mut pages = vec![page_with_extra("post", "one two", extra)];

        evaluate_computed_fields(&mut pages);

        let extra = pages[0].frontmatter.extra.as_ref().unwrap();
        assert_eq!(extra["author"], "Alice"); // unchanged
        assert_eq!(extra["computed"], serde_json::json!(2)); // computed
    }

    #[test]
    fn invalid_expression_warns_but_doesnt_crash() {
        let mut extra = HashMap::new();
        extra.insert(
            "bad".to_string(),
            serde_json::Value::String("rhai: undefined_var + 1".to_string()),
        );
        let mut pages = vec![page_with_extra("post", "content", extra)];

        // Should not panic
        evaluate_computed_fields(&mut pages);
        // The bad field should still have some value (original or error)
    }

    #[test]
    fn no_extra_is_noop() {
        let mut pages = vec![Page {
            source_path: PathBuf::from("test.md"),
            slug: "test".to_string(),
            frontmatter: Frontmatter {
                title: "Test".into(),
                ..Default::default()
            },
            raw_content: "content".to_string(),
            rendered_html: None,
            output_path: None,
            content_hash: 0,
            toc: Vec::new(),
        }];

        evaluate_computed_fields(&mut pages);
        assert!(pages[0].frontmatter.extra.is_none());
    }
}
