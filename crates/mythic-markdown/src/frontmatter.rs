//! Frontmatter parsing supporting both YAML and TOML delimiters.

use anyhow::{bail, Context, Result};
use mythic_core::page::Frontmatter;

/// Parse frontmatter and body from raw file content.
///
/// Supports YAML (`---`) and TOML (`+++`) delimited frontmatter.
/// Returns the parsed frontmatter and the remaining body content.
pub fn parse_frontmatter(raw: &str) -> Result<(Frontmatter, String)> {
    if raw.starts_with("---") {
        parse_yaml_frontmatter(raw)
    } else if raw.starts_with("+++") {
        parse_toml_frontmatter(raw)
    } else {
        bail!("No frontmatter found: content must begin with `---` (YAML) or `+++` (TOML)")
    }
}

fn parse_yaml_frontmatter(raw: &str) -> Result<(Frontmatter, String)> {
    let after_open = &raw[3..];
    let close_pos = after_open
        .find("\n---")
        .context("Unclosed YAML frontmatter: missing closing `---`")?;

    let yaml_str = &after_open[..close_pos];
    // +1 to skip the newline before ---
    let body_start = 3 + close_pos + 4; // "---" opener (3) + content + "\n---" (4)
    let body = if body_start < raw.len() {
        raw[body_start..].trim_start().to_string()
    } else {
        String::new()
    };

    let fm: Frontmatter =
        serde_yaml::from_str(yaml_str).context("Failed to parse YAML frontmatter")?;
    Ok((fm, body))
}

fn parse_toml_frontmatter(raw: &str) -> Result<(Frontmatter, String)> {
    let after_open = &raw[3..];
    let close_pos = after_open
        .find("\n+++")
        .context("Unclosed TOML frontmatter: missing closing `+++`")?;

    let toml_str = &after_open[..close_pos];
    let body_start = 3 + close_pos + 4;
    let body = if body_start < raw.len() {
        raw[body_start..].trim_start().to_string()
    } else {
        String::new()
    };

    let fm: Frontmatter =
        toml::from_str(toml_str).context("Failed to parse TOML frontmatter")?;
    Ok((fm, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_happy_path() {
        let input = "---\ntitle: Hello World\ndate: \"2024-01-15\"\ntags:\n  - rust\n  - web\n---\n# Hello\n\nBody content here.";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title, "Hello World");
        assert_eq!(fm.date.as_deref(), Some("2024-01-15"));
        assert_eq!(fm.tags.as_ref().unwrap(), &["rust", "web"]);
        assert!(body.starts_with("# Hello"));
    }

    #[test]
    fn toml_happy_path() {
        let input = "+++\ntitle = \"TOML Post\"\ndraft = true\n+++\nSome body text.";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title, "TOML Post");
        assert_eq!(fm.draft, Some(true));
        assert_eq!(body, "Some body text.");
    }

    #[test]
    fn missing_optional_fields() {
        let input = "---\ntitle: Minimal\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title, "Minimal");
        assert!(fm.date.is_none());
        assert!(fm.draft.is_none());
        assert!(fm.tags.is_none());
    }

    #[test]
    fn unclosed_yaml_delimiter() {
        let input = "---\ntitle: Broken\nNo closing delimiter";
        assert!(parse_frontmatter(input).is_err());
    }

    #[test]
    fn no_frontmatter_errors() {
        let input = "# Just a heading\n\nSome content.";
        assert!(parse_frontmatter(input).is_err());
    }

    #[test]
    fn default_layout_is_set() {
        let input = "---\ntitle: No Layout\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.layout.as_deref(), Some("default"));
    }

    #[test]
    fn custom_layout() {
        let input = "---\ntitle: Post\nlayout: blog\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.layout.as_deref(), Some("blog"));
    }

    #[test]
    fn draft_true() {
        let input = "---\ntitle: Draft\ndraft: true\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.draft, Some(true));
    }

    #[test]
    fn draft_false() {
        let input = "---\ntitle: Published\ndraft: false\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.draft, Some(false));
    }

    #[test]
    fn extra_fields_preserved() {
        let input = "---\ntitle: Extra\nextra:\n  author: Alice\n  featured: true\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        let extra = fm.extra.unwrap();
        assert_eq!(extra["author"], "Alice");
        assert_eq!(extra["featured"], true);
    }

    #[test]
    fn empty_tags_list() {
        let input = "---\ntitle: No Tags\ntags: []\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert!(fm.tags.unwrap().is_empty());
    }

    #[test]
    fn body_preserves_content() {
        let input = "---\ntitle: Test\n---\n\n# Heading\n\nParagraph with **bold**.\n\n- list item\n";
        let (_, body) = parse_frontmatter(input).unwrap();
        assert!(body.contains("# Heading"));
        assert!(body.contains("**bold**"));
        assert!(body.contains("- list item"));
    }

    #[test]
    fn yaml_with_multiline_string() {
        let input = "---\ntitle: \"Multi: line\"\ndate: \"2024-01-15\"\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title, "Multi: line");
    }

    #[test]
    fn toml_with_all_fields() {
        let input = "+++\ntitle = \"TOML All\"\ndate = \"2024-06-15\"\ndraft = false\nlayout = \"post\"\ntags = [\"a\", \"b\"]\n+++\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title, "TOML All");
        assert_eq!(fm.date.as_deref(), Some("2024-06-15"));
        assert_eq!(fm.draft, Some(false));
        assert_eq!(fm.layout.as_deref(), Some("post"));
        assert_eq!(fm.tags.unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn unclosed_toml_delimiter() {
        let input = "+++\ntitle = \"Broken\"\nNo closing delimiter";
        assert!(parse_frontmatter(input).is_err());
    }

    #[test]
    fn empty_body_after_frontmatter() {
        let input = "---\ntitle: Empty\n---\n";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title, "Empty");
        assert!(body.is_empty());
    }

    #[test]
    fn frontmatter_with_special_characters() {
        let input = "---\ntitle: \"Hello & <World>\"\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title, "Hello & <World>");
    }

    #[test]
    fn sitemap_field() {
        let input = "---\ntitle: Hidden\nsitemap: false\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.sitemap, Some(false));
    }

    #[test]
    fn locale_field() {
        let input = "---\ntitle: Spanish\nlocale: es\n---\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.locale.as_deref(), Some("es"));
    }
}
