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

    // Transform display math ($$...$$) — must come before inline
    result = transform_display_math(&result);

    // Transform inline math ($...$)
    result = transform_inline_math(&result);

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
}
