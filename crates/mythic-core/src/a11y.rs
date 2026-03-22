//! Build-time accessibility auditing for generated HTML.
//!
//! Checks generated HTML for common WCAG violations and reports
//! them as warnings during the build process.

use std::path::Path;
use walkdir::WalkDir;

/// Severity level for accessibility issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A11ySeverity {
    Error,
    Warning,
}

/// A single accessibility issue found in a page.
#[derive(Debug)]
pub struct A11yIssue {
    pub file: String,
    pub severity: A11ySeverity,
    pub rule: String,
    pub message: String,
}

/// Result of an accessibility audit.
#[derive(Debug, Default)]
pub struct A11yReport {
    pub issues: Vec<A11yIssue>,
    pub pages_checked: usize,
}

impl A11yReport {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|i| i.severity == A11ySeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == A11ySeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == A11ySeverity::Warning)
            .count()
    }
}

/// Run accessibility checks on all HTML files in the output directory.
pub fn audit_site(output_dir: &Path) -> A11yReport {
    let mut report = A11yReport::default();

    let html_files: Vec<_> = WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().and_then(|x| x.to_str()) == Some("html")
        })
        .collect();

    for entry in &html_files {
        let path = entry.path();
        let rel = path
            .strip_prefix(output_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        if let Ok(html) = std::fs::read_to_string(path) {
            audit_html(&html, &rel, &mut report);
            report.pages_checked += 1;
        }
    }

    report
}

fn audit_html(html: &str, file: &str, report: &mut A11yReport) {
    check_images_alt(html, file, report);
    check_lang_attribute(html, file, report);
    check_heading_order(html, file, report);
    check_empty_links(html, file, report);
    check_form_labels(html, file, report);
    check_color_contrast_hints(html, file, report);
    check_meta_viewport(html, file, report);
}

/// Check that all <img> tags have alt attributes.
fn check_images_alt(html: &str, file: &str, report: &mut A11yReport) {
    let mut rest = html;
    while let Some(start) = rest.find("<img") {
        let after = &rest[start..];
        let tag_end = after.find('>').unwrap_or(after.len());
        let tag = &after[..tag_end + 1];

        if !tag.contains("alt=") && !tag.contains("alt =") {
            // Decorative images can use alt="" — check for missing attribute entirely
            report.issues.push(A11yIssue {
                file: file.to_string(),
                severity: A11ySeverity::Error,
                rule: "img-alt".to_string(),
                message: "Image missing alt attribute".to_string(),
            });
        } else if tag.contains("alt=\"\"") {
            // Empty alt is valid for decorative images, but warn
            report.issues.push(A11yIssue {
                file: file.to_string(),
                severity: A11ySeverity::Warning,
                rule: "img-alt-empty".to_string(),
                message: "Image has empty alt attribute (OK for decorative images only)"
                    .to_string(),
            });
        }

        rest = &rest[start + tag_end + 1..];
    }
}

/// Check that <html> has a lang attribute.
fn check_lang_attribute(html: &str, file: &str, report: &mut A11yReport) {
    if html.contains("<html") && !html.contains("lang=") {
        report.issues.push(A11yIssue {
            file: file.to_string(),
            severity: A11ySeverity::Error,
            rule: "html-lang".to_string(),
            message: "<html> element missing lang attribute".to_string(),
        });
    }
}

/// Check heading order (no skipping levels).
fn check_heading_order(html: &str, file: &str, report: &mut A11yReport) {
    let mut last_level: Option<u32> = None;
    let bytes = html.as_bytes();

    for i in 0..bytes.len().saturating_sub(3) {
        if bytes[i] == b'<'
            && bytes[i + 1] == b'h'
            && bytes[i + 2].is_ascii_digit()
            && (i + 3 >= bytes.len() || bytes[i + 3] == b'>' || bytes[i + 3] == b' ')
        {
            let level = (bytes[i + 2] - b'0') as u32;
            if !(1..=6).contains(&level) {
                continue;
            }
            if i > 0 && bytes[i.saturating_sub(1)] == b'/' {
                continue;
            }

            if let Some(prev) = last_level {
                if level > prev + 1 {
                    report.issues.push(A11yIssue {
                        file: file.to_string(),
                        severity: A11ySeverity::Warning,
                        rule: "heading-order".to_string(),
                        message: format!(
                            "Heading level skipped: h{prev} → h{level} (missing h{})",
                            prev + 1
                        ),
                    });
                }
            }
            last_level = Some(level);
        }
    }
}

/// Check for empty links (<a> with no text content).
fn check_empty_links(html: &str, file: &str, report: &mut A11yReport) {
    let mut rest = html;
    while let Some(start) = rest.find("<a ") {
        let after = &rest[start..];
        if let Some(close) = after.find("</a>") {
            if let Some(open_end) = after.find('>') {
                let text = &after[open_end + 1..close];
                let stripped = text.replace(|c: char| c.is_whitespace(), "");
                // Empty link (no text, no aria-label, no img)
                if stripped.is_empty()
                    && !after[..close].contains("aria-label")
                    && !after[..close].contains("<img")
                {
                    report.issues.push(A11yIssue {
                        file: file.to_string(),
                        severity: A11ySeverity::Error,
                        rule: "link-text".to_string(),
                        message: "Link has no text content".to_string(),
                    });
                }
            }
            rest = &after[close + 4..];
        } else {
            break;
        }
    }
}

/// Check that form inputs have associated labels.
fn check_form_labels(html: &str, file: &str, report: &mut A11yReport) {
    let mut rest = html;
    while let Some(start) = rest.find("<input") {
        let after = &rest[start..];
        let tag_end = after.find('>').unwrap_or(after.len());
        let tag = &after[..tag_end + 1];

        // Skip hidden, submit, button types
        let is_skippable = tag.contains("type=\"hidden\"")
            || tag.contains("type=\"submit\"")
            || tag.contains("type=\"button\"")
            || tag.contains("type=\"checkbox\"")
            || tag.contains("type=\"radio\"");

        if !is_skippable && !tag.contains("aria-label") && !tag.contains("id=") {
            report.issues.push(A11yIssue {
                file: file.to_string(),
                severity: A11ySeverity::Warning,
                rule: "input-label".to_string(),
                message: "Input element may be missing a label".to_string(),
            });
        }

        rest = &rest[start + tag_end + 1..];
    }
}

/// Hint about potential color contrast issues.
fn check_color_contrast_hints(html: &str, file: &str, report: &mut A11yReport) {
    // Check for very light text colors in inline styles
    let light_colors = [
        "color:#ccc",
        "color:#ddd",
        "color:#eee",
        "color:#fff",
        "color: #ccc",
        "color: #ddd",
        "color: #eee",
        "color: #fff",
        "color:lightgray",
        "color: lightgray",
    ];

    for pattern in &light_colors {
        if html.contains(pattern) {
            report.issues.push(A11yIssue {
                file: file.to_string(),
                severity: A11ySeverity::Warning,
                rule: "color-contrast".to_string(),
                message: format!("Potential low contrast text detected ({})", pattern.trim()),
            });
            break;
        }
    }
}

/// Check for proper viewport meta tag.
fn check_meta_viewport(html: &str, file: &str, report: &mut A11yReport) {
    if html.contains("<head") && !html.contains("viewport") {
        report.issues.push(A11yIssue {
            file: file.to_string(),
            severity: A11ySeverity::Warning,
            rule: "meta-viewport".to_string(),
            message: "Missing viewport meta tag (affects mobile accessibility)".to_string(),
        });
    }

    if html.contains("maximum-scale=1") || html.contains("user-scalable=no") {
        report.issues.push(A11yIssue {
            file: file.to_string(),
            severity: A11ySeverity::Error,
            rule: "meta-viewport-scalable".to_string(),
            message: "Viewport disables user zooming (violates WCAG 1.4.4)".to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audit_str(html: &str) -> A11yReport {
        let mut report = A11yReport::default();
        audit_html(html, "test.html", &mut report);
        report.pages_checked = 1;
        report
    }

    #[test]
    fn missing_alt_is_error() {
        let report = audit_str("<img src=\"photo.jpg\">");
        assert!(report.has_errors());
        assert!(report.issues.iter().any(|i| i.rule == "img-alt"));
    }

    #[test]
    fn present_alt_passes() {
        let report = audit_str("<img src=\"photo.jpg\" alt=\"A photo\">");
        assert!(!report
            .issues
            .iter()
            .any(|i| i.rule == "img-alt" && i.severity == A11ySeverity::Error));
    }

    #[test]
    fn missing_lang_is_error() {
        let report = audit_str("<!DOCTYPE html><html><head></head><body></body></html>");
        assert!(report.issues.iter().any(|i| i.rule == "html-lang"));
    }

    #[test]
    fn present_lang_passes() {
        let report =
            audit_str("<!DOCTYPE html><html lang=\"en\"><head></head><body></body></html>");
        assert!(!report.issues.iter().any(|i| i.rule == "html-lang"));
    }

    #[test]
    fn heading_skip_detected() {
        let report = audit_str("<h1>Title</h1><h3>Skipped h2</h3>");
        assert!(report.issues.iter().any(|i| i.rule == "heading-order"));
    }

    #[test]
    fn proper_heading_order_passes() {
        let report = audit_str("<h1>A</h1><h2>B</h2><h3>C</h3>");
        assert!(!report.issues.iter().any(|i| i.rule == "heading-order"));
    }

    #[test]
    fn empty_link_detected() {
        let report = audit_str("<a href=\"/page\"> </a>");
        assert!(report.issues.iter().any(|i| i.rule == "link-text"));
    }

    #[test]
    fn link_with_text_passes() {
        let report = audit_str("<a href=\"/page\">Click here</a>");
        assert!(!report.issues.iter().any(|i| i.rule == "link-text"));
    }

    #[test]
    fn zoom_disabled_is_error() {
        let report = audit_str(
            "<html lang=\"en\"><head><meta name=\"viewport\" content=\"width=device-width, user-scalable=no\"></head><body></body></html>",
        );
        assert!(report
            .issues
            .iter()
            .any(|i| i.rule == "meta-viewport-scalable"));
    }

    #[test]
    fn missing_viewport_warned() {
        let report = audit_str(
            "<!DOCTYPE html><html lang=\"en\"><head><title>T</title></head><body></body></html>",
        );
        assert!(report.issues.iter().any(|i| i.rule == "meta-viewport"));
    }
}
