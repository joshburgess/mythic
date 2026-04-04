//! Link checking and content validation.

use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

/// Result of a site check.
#[derive(Debug, Default)]
pub struct CheckReport {
    pub errors: Vec<CheckIssue>,
    pub warnings: Vec<CheckIssue>,
    pub pages_checked: usize,
    pub links_checked: usize,
}

/// A single check issue.
#[derive(Debug)]
pub struct CheckIssue {
    pub file: String,
    pub message: String,
}

impl CheckReport {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl std::fmt::Display for CheckReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\nCheck results:")?;
        writeln!(f, "  Pages checked: {}", self.pages_checked)?;
        writeln!(f, "  Links checked: {}", self.links_checked)?;

        if !self.errors.is_empty() {
            writeln!(f, "\n  Errors ({}):", self.errors.len())?;
            for e in &self.errors {
                writeln!(f, "    {} — {}", e.file, e.message)?;
            }
        }

        if !self.warnings.is_empty() {
            writeln!(f, "\n  Warnings ({}):", self.warnings.len())?;
            for w in &self.warnings {
                writeln!(f, "    {} — {}", w.file, w.message)?;
            }
        }

        if self.errors.is_empty() && self.warnings.is_empty() {
            writeln!(f, "  No issues found.")?;
        }

        Ok(())
    }
}

/// Run all checks on the built output directory.
pub fn check_site(output_dir: &Path) -> Result<CheckReport> {
    let mut report = CheckReport::default();

    let html_files = discover_html_files(output_dir);

    for file in &html_files {
        let content = std::fs::read_to_string(file)?;
        let rel_path = file
            .strip_prefix(output_dir)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();

        report.pages_checked += 1;

        // Check internal links
        check_internal_links(&content, &rel_path, output_dir, &mut report);

        // Check images
        check_images(&content, &rel_path, &mut report);

        // Check heading hierarchy
        check_heading_hierarchy(&content, &rel_path, &mut report);
    }

    Ok(report)
}

fn discover_html_files(dir: &Path) -> Vec<std::path::PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("html"))
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn check_internal_links(
    html: &str,
    source_file: &str,
    output_dir: &Path,
    report: &mut CheckReport,
) {
    // Extract href and src attributes
    let links = extract_links(html);

    for link in links {
        report.links_checked += 1;

        // Skip external links, anchors, mailto, tel, javascript
        if link.starts_with("http://")
            || link.starts_with("https://")
            || link.starts_with('#')
            || link.starts_with("mailto:")
            || link.starts_with("tel:")
            || link.starts_with("javascript:")
        {
            continue;
        }

        // Resolve the link relative to output_dir
        let clean_link = link.split('#').next().unwrap_or(&link);
        let clean_link = clean_link.split('?').next().unwrap_or(clean_link);

        if clean_link.is_empty() {
            continue;
        }

        let target = if clean_link.starts_with('/') {
            output_dir.join(clean_link.trim_start_matches('/'))
        } else {
            // Relative link
            let parent = std::path::Path::new(source_file)
                .parent()
                .unwrap_or(std::path::Path::new(""));
            output_dir.join(parent).join(clean_link)
        };

        // Check if the target exists (try as file, directory/index.html)
        let exists = target.exists()
            || target.join("index.html").exists()
            || target.with_extension("html").exists();

        if !exists {
            report.errors.push(CheckIssue {
                file: source_file.to_string(),
                message: format!("Broken internal link: {link}"),
            });
        }
    }
}

fn check_images(html: &str, source_file: &str, report: &mut CheckReport) {
    // Find <img> tags and check for alt attributes
    let mut rest = html;

    while let Some(start) = rest.find("<img") {
        let after = &rest[start..];
        let tag_end = after.find('>').unwrap_or(after.len());
        let tag = &after[..tag_end + 1];

        if !tag.contains("alt=") && !tag.contains("alt =") {
            report.warnings.push(CheckIssue {
                file: source_file.to_string(),
                message: format!("Image missing alt attribute: {}", truncate_tag(tag)),
            });
        }

        rest = &rest[start + tag_end + 1..];
    }
}

fn check_heading_hierarchy(html: &str, source_file: &str, report: &mut CheckReport) {
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

            // Check if this is an opening tag (not </h>)
            if i > 0 && bytes[i - 1] == b'/' {
                continue;
            }

            if let Some(prev) = last_level {
                if level > prev + 1 {
                    report.warnings.push(CheckIssue {
                        file: source_file.to_string(),
                        message: format!(
                            "Heading hierarchy skip: h{prev} → h{level} (missing h{})",
                            prev + 1
                        ),
                    });
                }
            }

            last_level = Some(level);
        }
    }
}

fn extract_links(html: &str) -> Vec<String> {
    let mut links = Vec::new();

    for attr in &["href=\"", "src=\"", "href='", "src='"] {
        let mut rest = html;
        while let Some(start) = rest.find(attr) {
            let after = &rest[start + attr.len()..];
            let quote = if attr.ends_with('"') { '"' } else { '\'' };
            if let Some(end) = after.find(quote) {
                links.push(after[..end].to_string());
            }
            rest = &rest[start + attr.len()..];
        }
    }

    links
}

fn truncate_tag(tag: &str) -> String {
    if tag.len() > 80 {
        format!("{}...", &tag[..77])
    } else {
        tag.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_site(dir: &Path, files: &[(&str, &str)]) {
        for (path, content) in files {
            let full = dir.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        }
    }

    #[test]
    fn broken_internal_link_detected() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[("index.html", r#"<a href="/missing/">Link</a>"#)],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(report.has_errors());
        assert!(report.errors[0].message.contains("missing"));
    }

    #[test]
    fn valid_internal_link_passes() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[
                ("index.html", r#"<a href="/about/">Link</a>"#),
                ("about/index.html", "<p>About page</p>"),
            ],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn missing_alt_text_warning() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[(
                "index.html",
                r#"<img src="photo.jpg"><img src="ok.jpg" alt="OK">"#,
            )],
        );

        let report = check_site(dir.path()).unwrap();
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("alt"));
    }

    #[test]
    fn heading_skip_warning() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[("index.html", "<h1>Title</h1><h3>Skipped h2</h3>")],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(report
            .warnings
            .iter()
            .any(|w| w.message.contains("h1 → h3")));
    }

    #[test]
    fn external_links_skipped() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[(
                "index.html",
                r#"<a href="https://example.com">External</a>"#,
            )],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn proper_heading_hierarchy_ok() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[("index.html", "<h1>A</h1><h2>B</h2><h3>C</h3><h2>D</h2>")],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn anchor_links_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[(
                "index.html",
                "<a href=\"#section\">Jump</a><a href=\"#top\">Top</a>",
            )],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn mailto_links_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[(
                "index.html",
                r#"<a href="mailto:user@example.com">Email</a>"#,
            )],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn multiple_broken_links_in_same_file() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[(
                "index.html",
                r#"<a href="/gone1/">A</a><a href="/gone2/">B</a><a href="/gone3/">C</a>"#,
            )],
        );

        let report = check_site(dir.path()).unwrap();
        assert_eq!(report.errors.len(), 3);
        assert!(report.errors.iter().all(|e| e.file == "index.html"));
    }

    #[test]
    fn nested_output_directory_structure() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[
                ("blog/post/index.html", r#"<a href="/about/">Link</a>"#),
                ("about/index.html", "<p>About</p>"),
            ],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
        assert_eq!(report.pages_checked, 2);
    }

    #[test]
    fn file_with_no_links_produces_no_errors() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[("index.html", "<p>Just text, no links at all.</p>")],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
        assert!(report.warnings.is_empty());
        assert_eq!(report.links_checked, 0);
    }

    #[test]
    fn image_with_alt_attribute_passes() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[(
                "index.html",
                r#"<img src="photo.jpg" alt="A photo"><img src="logo.png" alt="Logo">"#,
            )],
        );

        let report = check_site(dir.path()).unwrap();
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn multiple_heading_skips_in_same_file() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[("index.html", "<h1>Title</h1><h3>Skip1</h3><h6>Skip2</h6>")],
        );

        let report = check_site(dir.path()).unwrap();
        // h1->h3 is a skip, h3->h6 is a skip
        assert!(report.warnings.len() >= 2);
    }

    #[test]
    fn valid_heading_going_back_up() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[(
                "index.html",
                "<h1>A</h1><h2>B</h2><h3>C</h3><h2>Back up</h2>",
            )],
        );

        let report = check_site(dir.path()).unwrap();
        // Going from h3 back to h2 is fine, no warnings
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn tel_links_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[("index.html", "<a href=\"tel:+1234567890\">Call</a>")],
        );
        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn javascript_links_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[("index.html", "<a href=\"javascript:void(0)\">Click</a>")],
        );
        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn link_with_query_string_resolved_correctly() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[
                ("index.html", "<a href=\"/about/?ref=nav\">About</a>"),
                ("about/index.html", "<p>About</p>"),
            ],
        );
        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }

    #[test]
    fn link_with_fragment_resolved_correctly() {
        let dir = tempfile::tempdir().unwrap();
        setup_site(
            dir.path(),
            &[
                ("index.html", "<a href=\"/about/#team\">Team</a>"),
                ("about/index.html", "<p>About</p>"),
            ],
        );
        let report = check_site(dir.path()).unwrap();
        assert!(!report.has_errors());
    }
}
