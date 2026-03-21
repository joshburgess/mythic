//! Syntax highlighting using syntect with CSS class output.

use syntect::highlighting::ThemeSet;
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;

/// Highlighter wrapping syntect's syntax and theme sets.
pub struct Highlighter {
    ss: SyntaxSet,
    theme_name: String,
    line_numbers: bool,
}

impl Highlighter {
    pub fn new(theme: &str, line_numbers: bool) -> Self {
        Highlighter {
            ss: SyntaxSet::load_defaults_newlines(),
            theme_name: theme.to_string(),
            line_numbers,
        }
    }

    /// Highlight a code block, returning HTML with CSS classes.
    pub fn highlight(&self, code: &str, lang: &str) -> String {
        let syntax = self
            .ss
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.ss.find_syntax_plain_text());

        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, &self.ss, ClassStyle::Spaced);

        for line in code.lines() {
            // Ignore errors from unparseable lines
            let _ = generator.parse_html_for_line_which_includes_newline(&format!("{line}\n"));
        }

        let highlighted = generator.finalize();

        if self.line_numbers {
            let lines: Vec<&str> = highlighted.lines().collect();
            let mut out = String::new();
            out.push_str("<pre><code>");
            for (i, line) in lines.iter().enumerate() {
                out.push_str(&format!(
                    "<span class=\"line-number\">{}</span>{}\n",
                    i + 1,
                    line
                ));
            }
            out.push_str("</code></pre>");
            out
        } else {
            format!("<pre><code>{highlighted}</code></pre>")
        }
    }

    /// Generate a CSS stylesheet for the configured theme.
    pub fn generate_css(&self) -> String {
        let ts = ThemeSet::load_defaults();
        let theme = ts
            .themes
            .get(&self.theme_name)
            .or_else(|| ts.themes.get("base16-ocean.dark"))
            .expect("No theme found");

        syntect::html::css_for_theme_with_class_style(theme, ClassStyle::Spaced)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_rust_code() {
        let h = Highlighter::new("base16-ocean.dark", false);
        let html = h.highlight("fn main() {\n    println!(\"hello\");\n}", "rust");
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("<span"));
        assert!(html.contains("main"));
    }

    #[test]
    fn plain_code_no_language() {
        let h = Highlighter::new("base16-ocean.dark", false);
        let html = h.highlight("just plain text", "txt");
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("just plain text"));
    }

    #[test]
    fn line_numbers() {
        let h = Highlighter::new("base16-ocean.dark", true);
        let html = h.highlight("line1\nline2\nline3", "rust");
        assert!(html.contains("line-number"));
        assert!(html.contains(">1<"));
        assert!(html.contains(">2<"));
        assert!(html.contains(">3<"));
    }

    #[test]
    fn generates_css() {
        let h = Highlighter::new("base16-ocean.dark", false);
        let css = h.generate_css();
        assert!(!css.is_empty());
        assert!(css.contains("color:"));
    }
}
