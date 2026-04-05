//! Math rendering support for LaTeX expressions.
//!
//! Transforms inline math (`$...$`) and display math (`$$...$$`) into
//! HTML elements suitable for client-side KaTeX or MathJax rendering.
//! Also transforms `math` code blocks into display math.

/// Transform LaTeX math expressions in HTML to KaTeX-compatible markup.
///
/// - `$$...$$` → `<div class="math math-display">...</div>`
/// - `$...$` → `<span class="math math-inline">...</span>`
/// - ````math` code blocks → `<div class="math math-display">...</div>`
pub fn transform_math(html: &str) -> String {
    let mut result = html.to_string();

    // Transform math code blocks: <pre><code class="language-math">...</code></pre>
    result = transform_math_code_blocks(&result);

    // Protect <code>...</code> and <pre>...</pre> from math processing
    let (protected, code_spans) = protect_code_elements(&result);

    // Transform display math ($$...$$) — must come before inline
    let processed = transform_display_math(&protected);

    // Transform inline math ($...$)
    let processed = transform_inline_math(&processed);

    // Restore protected code elements
    result = restore_code_elements(&processed, &code_spans);

    result
}

/// Replace `<code>...</code>` and `<pre>...</pre>` with placeholders so math
/// processing doesn't touch their contents.
fn protect_code_elements(html: &str) -> (String, Vec<String>) {
    let mut protected = html.to_string();
    let mut spans: Vec<String> = Vec::new();

    // Protect <pre>...</pre> blocks first (they may contain <code> inside)
    while let Some(start) = protected.find("<pre") {
        if let Some(end_tag_start) = protected[start..].find("</pre>") {
            let end = start + end_tag_start + 6; // len("</pre>")
            let span = protected[start..end].to_string();
            let placeholder = format!("\x00CODEPROTECT{}\x00", spans.len());
            spans.push(span);
            protected = format!(
                "{}{}{}",
                &protected[..start],
                placeholder,
                &protected[end..]
            );
        } else {
            break;
        }
    }

    // Protect remaining <code>...</code> spans
    while let Some(start) = protected.find("<code") {
        if let Some(end_tag_start) = protected[start..].find("</code>") {
            let end = start + end_tag_start + 7; // len("</code>")
            let span = protected[start..end].to_string();
            let placeholder = format!("\x00CODEPROTECT{}\x00", spans.len());
            spans.push(span);
            protected = format!(
                "{}{}{}",
                &protected[..start],
                placeholder,
                &protected[end..]
            );
        } else {
            break;
        }
    }

    (protected, spans)
}

/// Restore placeholders with the original code elements.
fn restore_code_elements(html: &str, spans: &[String]) -> String {
    let mut result = html.to_string();
    for (i, span) in spans.iter().enumerate() {
        result = result.replace(&format!("\x00CODEPROTECT{}\x00", i), span);
    }
    result
}

fn transform_math_code_blocks(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    let open_pattern = "<pre><code class=\"language-math\">";
    let close_pattern = "</code></pre>";

    while let Some(start) = remaining.find(open_pattern) {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + open_pattern.len()..];

        if let Some(end) = after.find(close_pattern) {
            let math_content = &after[..end];
            result.push_str(&format!(
                "<div class=\"math math-display\">{math_content}</div>"
            ));
            remaining = &after[end + close_pattern.len()..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

fn transform_display_math(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find("$$") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + 2..];

        if let Some(end) = after.find("$$") {
            let math_content = &after[..end];
            result.push_str(&format!(
                "<div class=\"math math-display\">{math_content}</div>"
            ));
            remaining = &after[end + 2..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

fn transform_inline_math(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '$' && (i == 0 || chars[i - 1] != '\\') {
            // Look for closing $
            if let Some(end) = find_closing_dollar(&chars, i + 1) {
                let math: String = chars[i + 1..end].iter().collect();
                // Skip if it looks like a price ($10) rather than math
                if !math.is_empty()
                    && !math
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
                {
                    result.push_str(&format!("<span class=\"math math-inline\">{math}</span>"));
                    i = end + 1;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn find_closing_dollar(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if chars[i] == '$' && (i == start || chars[i - 1] != '\\') {
            return Some(i);
        }
        // Don't span across block elements
        if chars[i] == '\n' && i + 1 < chars.len() && chars[i + 1] == '\n' {
            return None;
        }
        i += 1;
    }
    None
}

/// Generate the KaTeX CSS/JS includes for the HTML head.
pub fn katex_head_tags() -> &'static str {
    r#"<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/contrib/auto-render.min.js" onload="renderMathInElement(document.body, {delimiters: [{left: '$$', right: '$$', display: true}, {left: '$', right: '$', display: false}]})"></script>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_math_transformed() {
        let html = "<p>Before $$E = mc^2$$ after</p>";
        let result = transform_math(html);
        assert!(result.contains("class=\"math math-display\""));
        assert!(result.contains("E = mc^2"));
    }

    #[test]
    fn inline_math_transformed() {
        let html = "<p>The equation $x^2 + y^2 = r^2$ is a circle.</p>";
        let result = transform_math(html);
        assert!(result.contains("class=\"math math-inline\""));
        assert!(result.contains("x^2 + y^2 = r^2"));
    }

    #[test]
    fn math_code_block_transformed() {
        let html = "<pre><code class=\"language-math\">\\int_0^1 x^2 dx</code></pre>";
        let result = transform_math(html);
        assert!(result.contains("class=\"math math-display\""));
        assert!(!result.contains("<pre>"));
        assert!(!result.contains("<code"));
    }

    #[test]
    fn dollar_prices_not_transformed() {
        // Single dollar amounts without a closing $ aren't math
        let html = "<p>The price is $10.</p>";
        let result = transform_math(html);
        assert!(result.contains("$10"));
    }

    #[test]
    fn no_math_unchanged() {
        let html = "<p>Just regular text.</p>";
        let result = transform_math(html);
        assert_eq!(result, html);
    }

    #[test]
    fn katex_head_tags_valid() {
        let tags = katex_head_tags();
        assert!(tags.contains("katex"));
        assert!(tags.contains("auto-render"));
    }

    #[test]
    fn math_inside_code_not_transformed() {
        let html = "<p>Use <code>$x^2$</code> for math.</p>";
        let result = transform_math(html);
        assert!(
            result.contains("<code>$x^2$</code>"),
            "Math inside <code> should not be transformed, got: {result}"
        );
    }

    #[test]
    fn math_inside_pre_not_transformed() {
        let html = "<pre>$$E = mc^2$$</pre>";
        let result = transform_math(html);
        assert!(
            result.contains("<pre>$$E = mc^2$$</pre>"),
            "Math inside <pre> should not be transformed, got: {result}"
        );
    }

    #[test]
    fn math_outside_code_still_transformed() {
        let html = "<p>$x^2$ and <code>$y^2$</code></p>";
        let result = transform_math(html);
        assert!(
            result.contains("<span class=\"math math-inline\">x^2</span>"),
            "Math outside <code> should be transformed, got: {result}"
        );
        assert!(
            result.contains("<code>$y^2$</code>"),
            "Math inside <code> should not be transformed, got: {result}"
        );
    }
}
