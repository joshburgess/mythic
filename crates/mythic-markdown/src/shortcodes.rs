//! Shortcode preprocessing for markdown content.
//!
//! Shortcodes use `{{% name arg="val" %}}` syntax for self-closing,
//! and `{{% name %}}content{{% /name %}}` for paired shortcodes.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tera::Tera;

/// A cached shortcode engine that avoids recreating the Tera instance per page.
pub struct ShortcodeEngine {
    tera: Tera,
}

impl ShortcodeEngine {
    /// Create a new shortcode engine by loading templates from the given directory.
    pub fn new(shortcode_dir: &Path) -> Self {
        if !shortcode_dir.exists() {
            return Self {
                tera: Tera::default(),
            };
        }
        let glob = shortcode_dir.join("*.html");
        let tera = Tera::new(&glob.to_string_lossy()).unwrap_or_default();
        Self { tera }
    }

    /// Process all shortcodes in the raw content using the cached engine.
    pub fn process(&self, content: &str) -> Result<String> {
        process_with_engine(content, &self.tera)
    }
}

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
    // Protect fenced code blocks from shortcode processing by replacing
    // them with placeholders, processing shortcodes, then restoring.
    let (protected, code_blocks) = extract_code_blocks(content);
    let mut result = protected;

    // Process paired shortcodes first (they may contain self-closing ones)
    const MAX_SHORTCODE_DEPTH: usize = 10;
    let mut iterations = 0;
    loop {
        if iterations >= MAX_SHORTCODE_DEPTH {
            anyhow::bail!(
                "Shortcode expansion exceeded maximum depth of {MAX_SHORTCODE_DEPTH} — possible circular reference"
            );
        }
        let before = result.clone();
        result = process_paired(&result, tera)?;
        if result == before {
            break;
        }
        iterations += 1;
    }

    // Then self-closing shortcodes
    result = process_self_closing(&result, tera)?;

    // Restore code blocks
    result = restore_code_blocks(&result, &code_blocks);

    Ok(result)
}

/// Count the number of consecutive backticks starting at the beginning of `s`.
fn count_backticks(s: &str) -> usize {
    s.bytes().take_while(|&b| b == b'`').count()
}

/// Extract fenced code blocks and replace with placeholders.
/// Handles fences of 3 or more backticks; the closing fence must have
/// at least as many backticks as the opening fence.
fn extract_code_blocks(content: &str) -> (String, Vec<String>) {
    let mut protected = String::with_capacity(content.len());
    let mut blocks = Vec::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("```") {
        protected.push_str(&remaining[..start]);

        // Count opening backticks (3 or more)
        let fence_len = count_backticks(&remaining[start..]);
        let fence_str = &remaining[start..start + fence_len];

        let after_open = &remaining[start + fence_len..];
        // Find a closing fence: newline followed by at least `fence_len` backticks
        let closing_pattern = format!("\n{fence_str}");
        if let Some(close) = after_open.find(&closing_pattern) {
            // The closing fence may have more backticks; consume them all
            let close_start = close + 1; // skip the newline
            let close_backticks = count_backticks(&after_open[close_start..]);
            let block_end = start + fence_len + close + 1 + close_backticks;
            let full_block = &remaining[start..block_end];
            let placeholder = format!("\x00CODE_BLOCK_{}\x00", blocks.len());
            blocks.push(full_block.to_string());
            protected.push_str(&placeholder);
            remaining = &remaining[block_end..];
        } else {
            // Unclosed code block — include the rest as-is
            protected.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    protected.push_str(remaining);
    (protected, blocks)
}

/// Restore code blocks from placeholders.
fn restore_code_blocks(content: &str, blocks: &[String]) -> String {
    let mut result = content.to_string();
    for (i, block) in blocks.iter().enumerate() {
        let placeholder = format!("\x00CODE_BLOCK_{i}\x00");
        result = result.replace(&placeholder, block);
    }
    result
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

        if let Some(after_quote) = after_eq.strip_prefix('"') {
            if let Some(close_quote) = after_quote.find('"') {
                let value = &after_quote[..close_quote];
                args.insert(key.to_string(), value.to_string());
                remaining = &after_quote[close_quote + 1..];
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
        assert!(
            result.contains("<strong>Important</strong>"),
            "Got: {result}"
        );
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

    #[test]
    fn multiple_self_closing_shortcodes() {
        let tera = setup_tera(&[("img.html", r#"<img src="{{ src }}" alt="{{ alt }}">"#)]);

        let input = r#"Before {{% img src="a.jpg" alt="A" %}} middle {{% img src="b.jpg" alt="B" %}} after"#;
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("src=\"a.jpg\""));
        assert!(result.contains("src=\"b.jpg\""));
        assert!(result.contains("Before"));
        assert!(result.contains("middle"));
        assert!(result.contains("after"));
    }

    #[test]
    fn shortcode_with_multiple_args() {
        let tera = setup_tera(&[(
            "video.html",
            r#"<video src="{{ src }}" width="{{ width }}" autoplay="{{ autoplay }}"></video>"#,
        )]);

        let input = r#"{{% video src="clip.mp4" width="640" autoplay="true" %}}"#;
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("src=\"clip.mp4\""));
        assert!(result.contains("width=\"640\""));
        assert!(result.contains("autoplay=\"true\""));
    }

    #[test]
    fn paired_shortcode_with_markdown_inner() {
        let tera = setup_tera(&[(
            "details.html",
            r#"<details><summary>{{ summary }}</summary>{{ inner | safe }}</details>"#,
        )]);

        let input = r#"{{% details summary="Click me" %}}**Bold** content with [link](https://example.com){{% /details %}}"#;
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("<summary>Click me</summary>"));
        assert!(result.contains("**Bold** content"));
    }

    #[test]
    fn shortcode_surrounded_by_markdown() {
        let tera = setup_tera(&[("hr.html", "<hr class=\"fancy\">")]);

        let input = "# Heading\n\nParagraph before.\n\n{{% hr %}}\n\nParagraph after.";
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("# Heading"));
        assert!(result.contains("<hr class=\"fancy\">"));
        assert!(result.contains("Paragraph after."));
    }

    #[test]
    fn error_message_includes_shortcode_name() {
        let tera = Tera::default();
        let input = "{{% missing_shortcode %}}";
        let err = process_with_engine(input, &tera).unwrap_err();
        assert!(err.to_string().contains("missing_shortcode"));
    }

    #[test]
    fn shortcode_in_code_block_not_expanded() {
        // Zola issue #1514: shortcodes inside fenced code blocks must be
        // preserved as literal text, not expanded.
        let tera = setup_tera(&[("youtube.html", "<iframe></iframe>")]);
        let input = "```\n{{% youtube id=\"abc\" %}}\n```";
        let result = process_with_engine(input, &tera).unwrap();
        // Shortcode syntax should be preserved inside code block
        assert!(
            result.contains("{{% youtube"),
            "Shortcode inside code block should not be expanded, got: {result}"
        );
        assert!(
            !result.contains("<iframe>"),
            "Shortcode template should NOT be rendered inside code block"
        );
    }

    #[test]
    fn shortcode_with_empty_body_renders() {
        // Zola issue #2564: empty body shortcodes should still render
        let tera = setup_tera(&[(
            "wrapper.html",
            "<div class=\"wrap\">{{ inner | safe }}</div>",
        )]);
        let input = "{{% wrapper %}}{{% /wrapper %}}";
        let result = process_with_engine(input, &tera).unwrap();
        assert!(result.contains("class=\"wrap\""));
        assert!(result.contains("<div class=\"wrap\"></div>"));
    }

    #[test]
    fn iteration_limit_prevents_infinite_loop() {
        // Create a paired shortcode whose template output contains another
        // paired shortcode invocation of itself. The engine re-processes
        // paired shortcodes in a loop, so this creates infinite expansion.
        // Use Tera's {% raw %} block so that Tera outputs the literal
        // shortcode syntax without trying to parse it.
        let tera = setup_tera(&[(
            "wrap.html",
            "<div>{% raw %}{{% wrap %}}{% endraw %}{{ inner | safe }}{% raw %}{{% /wrap %}}{% endraw %}</div>",
        )]);

        let input = "{{% wrap %}}seed{{% /wrap %}}";
        let result = process_with_engine(input, &tera);
        assert!(
            result.is_err(),
            "Recursive shortcode expansion should produce an error"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("maximum depth") || err_msg.contains("circular"),
            "Error message should mention depth limit, got: {err_msg}"
        );
    }

    #[test]
    fn shortcode_in_four_backtick_code_block_not_expanded() {
        let tera = setup_tera(&[("youtube.html", "<iframe></iframe>")]);
        let input = "````\n{{% youtube id=\"abc\" %}}\n````";
        let result = process_with_engine(input, &tera).unwrap();
        assert!(
            result.contains("{{% youtube"),
            "Shortcode inside 4-backtick code block should not be expanded, got: {result}"
        );
        assert!(
            !result.contains("<iframe>"),
            "Shortcode template should NOT be rendered inside 4-backtick code block"
        );
    }

    #[test]
    fn shortcode_in_five_backtick_code_block_not_expanded() {
        let tera = setup_tera(&[("youtube.html", "<iframe></iframe>")]);
        let input = "`````\n{{% youtube id=\"abc\" %}}\n`````";
        let result = process_with_engine(input, &tera).unwrap();
        assert!(
            result.contains("{{% youtube"),
            "Shortcode inside 5-backtick code block should not be expanded, got: {result}"
        );
    }

    #[test]
    fn extract_code_blocks_triple_backtick() {
        let (protected, blocks) = extract_code_blocks("before\n```\ncode\n```\nafter");
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("code"));
        assert!(protected.contains("after"));
    }

    #[test]
    fn extract_code_blocks_quad_backtick() {
        let (protected, blocks) = extract_code_blocks("before\n````\ncode with ```\n````\nafter");
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("code with ```"));
        assert!(protected.contains("after"));
    }
}
