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
}
