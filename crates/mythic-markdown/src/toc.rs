//! Table of contents extraction from rendered HTML.

use mythic_core::page::TocEntry;
use std::collections::HashMap;

/// Extract headings from HTML and return TOC entries and modified HTML with IDs.
pub fn extract_toc(html: &str, min_level: u32, max_level: u32) -> (Vec<TocEntry>, String) {
    let mut entries = Vec::new();
    let mut id_counts: HashMap<String, usize> = HashMap::new();
    let mut modified = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(tag_start) = find_heading_tag(remaining) {
        let before = &remaining[..tag_start];
        modified.push_str(before);

        let after_open = &remaining[tag_start..];

        // Parse the heading level (h1-h6)
        let level = match after_open.as_bytes().get(2) {
            Some(b) if b.is_ascii_digit() => (*b - b'0') as u32,
            _ => {
                modified.push_str(&after_open[..1]);
                remaining = &after_open[1..];
                continue;
            }
        };

        // Find the closing tag
        let close_tag = format!("</h{level}>");
        let Some(close_pos) = after_open.find(&close_tag) else {
            modified.push_str(&after_open[..1]);
            remaining = &after_open[1..];
            continue;
        };

        // Find end of opening tag
        let Some(tag_end) = after_open.find('>') else {
            modified.push_str(&after_open[..1]);
            remaining = &after_open[1..];
            continue;
        };

        let inner_html = &after_open[tag_end + 1..close_pos];
        let text = strip_html(inner_html);
        let base_id = slugify_heading(&text);

        // Handle duplicate IDs
        let id = {
            let count = id_counts.entry(base_id.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                base_id
            } else {
                format!("{base_id}-{}", *count - 1)
            }
        };

        if level >= min_level && level <= max_level {
            entries.push(TocEntry {
                level,
                text: text.clone(),
                id: id.clone(),
            });
        }

        // Write heading with id attribute
        let opening_tag = &after_open[..tag_end];
        if opening_tag.contains("id=") {
            modified.push_str(&after_open[..close_pos + close_tag.len()]);
        } else {
            modified.push_str(&format!("<h{level} id=\"{}\">", escape_html(&id)));
            modified.push_str(inner_html);
            modified.push_str(&close_tag);
        }

        remaining = &after_open[close_pos + close_tag.len()..];
    }

    modified.push_str(remaining);
    (entries, modified)
}

/// Render a TOC as nested HTML navigation.
pub fn render_toc_html(entries: &[TocEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut html = String::from("<nav class=\"toc\">\n");
    let mut current_level = 0u32;
    let mut list_depth = 0u32;

    for entry in entries {
        while current_level < entry.level {
            // Wrap nested <ul> in an <li> to produce valid HTML when levels skip
            if current_level > 0 {
                html.push_str("<li>\n");
            }
            html.push_str("<ul>\n");
            current_level += 1;
            list_depth += 1;
        }
        while current_level > entry.level {
            html.push_str("</ul>\n");
            html.push_str("</li>\n");
            current_level -= 1;
            list_depth -= 1;
        }
        html.push_str(&format!(
            "<li><a href=\"#{}\">{}</a></li>\n",
            escape_html(&entry.id),
            escape_html(&entry.text)
        ));
    }

    while list_depth > 0 {
        html.push_str("</ul>\n");
        if list_depth > 1 {
            html.push_str("</li>\n");
        }
        list_depth -= 1;
    }

    html.push_str("</nav>");
    html
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn find_heading_tag(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(3) {
        if bytes[i] == b'<'
            && bytes[i + 1] == b'h'
            && bytes[i + 2].is_ascii_digit()
            && (bytes[i + 3] == b'>' || bytes[i + 3] == b' ')
        {
            let level = bytes[i + 2] - b'0';
            if (1..=6).contains(&level) {
                return Some(i);
            }
        }
    }
    None
}

fn strip_html(s: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(c);
        }
    }
    text
}

fn slugify_heading(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_toc_extraction() {
        let html =
            "<h1>Title</h1>\n<p>text</p>\n<h2>Section A</h2>\n<p>more</p>\n<h2>Section B</h2>";
        let (entries, modified) = extract_toc(html, 1, 6);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "Title");
        assert_eq!(entries[0].level, 1);
        assert_eq!(entries[0].id, "title");
        assert_eq!(entries[1].text, "Section A");
        assert_eq!(entries[2].text, "Section B");

        assert!(modified.contains("id=\"title\""));
        assert!(modified.contains("id=\"section-a\""));
    }

    #[test]
    fn duplicate_id_handling() {
        let html = "<h2>FAQ</h2>\n<h2>FAQ</h2>\n<h2>FAQ</h2>";
        let (entries, modified) = extract_toc(html, 1, 6);

        assert_eq!(entries[0].id, "faq");
        assert_eq!(entries[1].id, "faq-1");
        assert_eq!(entries[2].id, "faq-2");

        assert!(modified.contains("id=\"faq\""));
        assert!(modified.contains("id=\"faq-1\""));
        assert!(modified.contains("id=\"faq-2\""));
    }

    #[test]
    fn min_max_level_filtering() {
        let html = "<h1>Title</h1>\n<h2>Section</h2>\n<h3>Sub</h3>\n<h5>Deep</h5>";
        let (entries, _) = extract_toc(html, 2, 4);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "Section");
        assert_eq!(entries[1].text, "Sub");
    }

    #[test]
    fn nested_toc_html() {
        let entries = vec![
            TocEntry {
                level: 2,
                text: "A".to_string(),
                id: "a".to_string(),
            },
            TocEntry {
                level: 3,
                text: "A.1".to_string(),
                id: "a-1".to_string(),
            },
            TocEntry {
                level: 2,
                text: "B".to_string(),
                id: "b".to_string(),
            },
        ];

        let html = render_toc_html(&entries);
        assert!(html.contains("<nav class=\"toc\">"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("href=\"#a\""));
        assert!(html.contains("href=\"#a-1\""));
        assert!(html.contains("href=\"#b\""));
    }

    #[test]
    fn headings_with_inline_code_clean_text() {
        let html = "<h2>Using <code>println!</code> in Rust</h2>";
        let (entries, _) = extract_toc(html, 1, 6);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Using println! in Rust");
    }

    #[test]
    fn headings_with_links_stripped() {
        let html = "<h2>See <a href=\"https://example.com\">Example Site</a> for details</h2>";
        let (entries, _) = extract_toc(html, 1, 6);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "See Example Site for details");
    }

    #[test]
    fn empty_document_produces_empty_toc() {
        let html = "";
        let (entries, modified) = extract_toc(html, 1, 6);
        assert!(entries.is_empty());
        assert!(modified.is_empty());
    }

    #[test]
    fn single_heading() {
        let html = "<h2>Only Section</h2>";
        let (entries, modified) = extract_toc(html, 1, 6);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Only Section");
        assert_eq!(entries[0].level, 2);
        assert_eq!(entries[0].id, "only-section");
        assert!(modified.contains("id=\"only-section\""));
    }

    #[test]
    fn headings_with_special_characters() {
        let html = "<h2>What's New in v2.0?</h2>\n<h2>C++ &amp; Rust</h2>";
        let (entries, _) = extract_toc(html, 1, 6);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "What's New in v2.0?");
        assert_eq!(entries[0].id, "what-s-new-in-v2-0");
        assert_eq!(entries[1].text, "C++ &amp; Rust");
    }

    #[test]
    fn render_toc_html_single_entry() {
        let entries = vec![TocEntry {
            level: 2,
            text: "Introduction".to_string(),
            id: "introduction".to_string(),
        }];
        let html = render_toc_html(&entries);
        assert!(html.contains("<nav class=\"toc\">"));
        assert!(html.contains("href=\"#introduction\""));
        assert!(html.contains("Introduction"));
        assert!(html.contains("</nav>"));
    }

    #[test]
    fn render_toc_html_empty_entries() {
        let entries: Vec<TocEntry> = vec![];
        let html = render_toc_html(&entries);
        assert!(html.is_empty());
    }

    #[test]
    fn heading_ids_handle_unicode() {
        let html = "<h2>Über Café résumé</h2>";
        let (entries, modified) = extract_toc(html, 1, 6);
        assert_eq!(entries.len(), 1);
        // slugify_heading keeps alphanumeric (including unicode) and hyphens
        let id = &entries[0].id;
        assert!(!id.is_empty());
        assert!(modified.contains(&format!("id=\"{id}\"")));
        // Verify the text is preserved
        assert_eq!(entries[0].text, "Über Café résumé");
    }

    #[test]
    fn heading_text_with_html_chars_escaped_in_toc() {
        // Heading text containing < > & " should be properly escaped in TOC output
        let entries = vec![TocEntry {
            level: 2,
            text: "A < B & C > D".to_string(),
            id: "a-b-c-d".to_string(),
        }];

        let html = render_toc_html(&entries);
        assert!(
            html.contains("A &lt; B &amp; C &gt; D"),
            "HTML special characters in TOC text should be escaped, got: {html}"
        );
        assert!(
            !html.contains("<li><a href=\"#a-b-c-d\">A < B"),
            "Unescaped < in TOC would break HTML"
        );
    }

    #[test]
    fn skipped_heading_levels_produce_valid_nesting() {
        // Document jumps from h2 to h4, skipping h3
        let entries = vec![
            TocEntry {
                level: 2,
                text: "Section".to_string(),
                id: "section".to_string(),
            },
            TocEntry {
                level: 4,
                text: "Deep".to_string(),
                id: "deep".to_string(),
            },
            TocEntry {
                level: 2,
                text: "Another".to_string(),
                id: "another".to_string(),
            },
        ];

        let html = render_toc_html(&entries);
        // Should produce valid nested HTML (opening and closing <ul> tags balance)
        let open_count = html.matches("<ul>").count();
        let close_count = html.matches("</ul>").count();
        assert_eq!(
            open_count, close_count,
            "Opening and closing <ul> tags should balance, got {open_count} opens and {close_count} closes in:\n{html}"
        );
        // All entries should be present
        assert!(html.contains("Section"));
        assert!(html.contains("Deep"));
        assert!(html.contains("Another"));
    }

    #[test]
    fn extract_toc_escapes_html_in_heading_ids() {
        // Heading with characters that need escaping in the id attribute
        let html = "<h2>Tom &amp; Jerry</h2>";
        let (entries, modified) = extract_toc(html, 1, 6);
        assert_eq!(entries.len(), 1);
        // The id should be in the modified HTML, properly quoted
        assert!(
            modified.contains("id=\""),
            "Modified HTML should contain an id attribute"
        );
        // The id value should not contain raw & or other dangerous chars
        let id = &entries[0].id;
        assert!(
            !id.contains('&') && !id.contains('<') && !id.contains('>'),
            "Heading id should not contain raw HTML special chars, got: {id}"
        );
    }
}
