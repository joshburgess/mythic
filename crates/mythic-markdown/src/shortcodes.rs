//! Shortcode preprocessing for markdown content.
//!
//! Shortcodes use `{{% name arg="val" %}}` syntax for self-closing,
//! and `{{% name %}}content{{% /name %}}` for paired shortcodes.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tera::Tera;

/// Process all shortcodes in the raw content, replacing them with rendered HTML.
pub fn process_shortcodes(content: &str, shortcode_dir: &Path) -> Result<String> {
    if !shortcode_dir.exists() {
        return Ok(content.to_string());
    }

    let glob = shortcode_dir.join("*.html");
    let tera = Tera::new(&glob.to_string_lossy()).unwrap_or_default();

    process_with_engine(content, &tera)
}

fn process_with_engine(content: &str, tera: &Tera) -> Result<String> {
    let mut result = content.to_string();

    // Process paired shortcodes first (they may contain self-closing ones)
    loop {
        let before = result.clone();
        result = process_paired(&result, tera)?;
        if result == before {
            break;
        }
    }

    // Then self-closing shortcodes
    result = process_self_closing(&result, tera)?;

    Ok(result)
}

fn process_paired(content: &str, tera: &Tera) -> Result<String> {
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    while let Some(open_start) = remaining.find("{{% ") {
        result.push_str(&remaining[..open_start]);
        let after_open = &remaining[open_start + 4..];

        // Find the closing %}
        let Some(open_end) = after_open.find(" %}}") else {
            result.push_str(&remaining[open_start..open_start + 4]);
            remaining = &remaining[open_start + 4..];
            continue;
        };

        let tag_content = &after_open[..open_end];
        let after_tag = &after_open[open_end + 4..];

        // Check if it's a closing tag
        if tag_content.starts_with('/') {
            result.push_str(&remaining[open_start..open_start + 4 + open_end + 4]);
            remaining = after_tag;
            continue;
        }

        let (name, args) = parse_shortcode_tag(tag_content);

        // Look for closing tag
        let close_tag = ["{{% /", &name, " %}}"].concat();
        if let Some(close_pos) = after_tag.find(&close_tag) {
            let raw_inner = &after_tag[..close_pos];
            let after_close = &after_tag[close_pos + close_tag.len()..];

            // Recursively process shortcodes in inner content
            let processed_inner = process_with_engine(raw_inner, tera)?;
            let rendered = render_shortcode(tera, &name, &args, Some(&processed_inner))?;
            result.push_str(&rendered);
            remaining = after_close;
        } else {
            // No closing tag — treat as self-closing
            let rendered = render_shortcode(tera, &name, &args, None)?;
            result.push_str(&rendered);
            remaining = after_tag;
        }
    }

    result.push_str(remaining);
    Ok(result)
}

fn process_self_closing(content: &str, tera: &Tera) -> Result<String> {
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    while let Some(start) = remaining.find("{{% ") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + 4..];

        let Some(end) = after.find(" %}}") else {
            result.push_str(&remaining[start..start + 4]);
            remaining = &remaining[start + 4..];
            continue;
        };

        let tag_content = &after[..end];

        // Skip closing tags
        if tag_content.starts_with('/') {
            result.push_str(&remaining[start..start + 4 + end + 4]);
            remaining = &after[end + 4..];
            continue;
        }

        let (name, args) = parse_shortcode_tag(tag_content);
        let rendered = render_shortcode(tera, &name, &args, None)?;
        result.push_str(&rendered);
        remaining = &after[end + 4..];
    }

    result.push_str(remaining);
    Ok(result)
}

fn parse_shortcode_tag(tag: &str) -> (String, HashMap<String, String>) {
    let mut parts = tag.splitn(2, ' ');
    let name = parts.next().unwrap_or("").trim().to_string();
    let args_str = parts.next().unwrap_or("");

    let mut args = HashMap::new();
    let mut remaining = args_str;

    while let Some(eq_pos) = remaining.find('=') {
        let key = remaining[..eq_pos].trim();
        let after_eq = remaining[eq_pos + 1..].trim();

        if after_eq.starts_with('"') {
            if let Some(close_quote) = after_eq[1..].find('"') {
                let value = &after_eq[1..1 + close_quote];
                args.insert(key.to_string(), value.to_string());
                remaining = &after_eq[1 + close_quote + 1..];
                continue;
            }
        }

        // Unquoted value — take until next space
        let value = after_eq.split_whitespace().next().unwrap_or("");
        args.insert(key.to_string(), value.to_string());
        remaining = &after_eq[value.len()..];
    }

    (name, args)
}

fn render_shortcode(
    tera: &Tera,
    name: &str,
    args: &HashMap<String, String>,
    inner: Option<&str>,
) -> Result<String> {
    let template_name = format!("{name}.html");

    if tera.get_template(&template_name).is_err() {
        bail!("Shortcode template not found: {template_name}");
    }

    let mut ctx = tera::Context::new();
    for (k, v) in args {
        ctx.insert(k, v);
    }
    if let Some(inner_content) = inner {
        ctx.insert("inner", inner_content);
    }

    tera.render(&template_name, &ctx)
        .with_context(|| format!("Failed to render shortcode: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_tera(templates: &[(&str, &str)]) -> Tera {
        let mut tera = Tera::default();
        for (name, content) in templates {
            tera.add_raw_template(name, content).unwrap();
        }
        tera
    }

    #[test]
    fn self_closing_shortcode() {
        let tera = setup_tera(&[(
            "youtube.html",
            r#"<iframe src="https://youtube.com/embed/{{ id }}"></iframe>"#,
        )]);

        let input = r#"Before {{% youtube id="abc123" %}} After"#;
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("youtube.com/embed/abc123"));
        assert!(result.contains("Before"));
        assert!(result.contains("After"));
    }

    #[test]
    fn paired_shortcode() {
        let tera = setup_tera(&[(
            "callout.html",
            r#"<div class="callout {{ type }}">{{ inner }}</div>"#,
        )]);

        let input = r#"{{% callout type="warning" %}}Be careful!{{% /callout %}}"#;
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("callout warning"));
        assert!(result.contains("Be careful!"));
    }

    #[test]
    fn nested_shortcodes() {
        let tera = setup_tera(&[
            ("bold.html", "<strong>{{ inner }}</strong>"),
            ("note.html", r#"<div class="note">{{ inner | safe }}</div>"#),
        ]);

        let input = r#"{{% note %}}{{% bold %}}Important{{% /bold %}}{{% /note %}}"#;
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("<strong>Important</strong>"), "Got: {result}");
        assert!(result.contains("class=\"note\""));
    }

    #[test]
    fn missing_template_error() {
        let tera = Tera::default();
        let input = "{{% nonexistent %}}";
        let result = process_with_engine(input, &tera);
        assert!(result.is_err());
    }

    #[test]
    fn no_shortcodes_passthrough() {
        let tera = Tera::default();
        let input = "Just regular markdown with no shortcodes.";
        let result = process_with_engine(input, &tera).unwrap();
        assert_eq!(result, input);
    }
}
