//! Build pipeline orchestration.

use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use crate::config::SiteConfig;
use crate::content::discover_content;
use crate::page::Page;

/// Summary of a completed build.
#[derive(Debug)]
pub struct BuildReport {
    pub total_pages: usize,
    pub pages_written: usize,
    pub pages_skipped: usize,
    pub elapsed_ms: u128,
}

/// Run the full build pipeline with pluggable render and template steps.
///
/// - `render_fn`: renders markdown to HTML on each page (mutates `rendered_html`)
/// - `template_fn`: applies a template to a page, returning the final HTML string
pub fn build<R, T>(
    config: &SiteConfig,
    root: &Path,
    include_drafts: bool,
    render_fn: R,
    template_fn: Option<T>,
) -> Result<BuildReport>
where
    R: Fn(&mut [Page]),
    T: Fn(&Page, &SiteConfig) -> Result<String>,
{
    let start = Instant::now();

    let mut pages = discover_content(config, root)?;

    // Filter drafts
    let pages_skipped = if !include_drafts {
        let before = pages.len();
        pages.retain(|p| !p.frontmatter.draft.unwrap_or(false));
        before - pages.len()
    } else {
        0
    };

    let total_pages = pages.len();

    // Render markdown
    render_fn(&mut pages);

    // Apply templates if provided
    if let Some(ref tmpl_fn) = template_fn {
        for page in &mut pages {
            let html = tmpl_fn(page, config)?;
            page.rendered_html = Some(html);
        }
    }

    // Write output files
    let output_dir = root.join(&config.output_dir);
    let pages_written = write_output(&pages, &output_dir)?;

    let elapsed_ms = start.elapsed().as_millis();

    let report = BuildReport {
        total_pages,
        pages_written,
        pages_skipped,
        elapsed_ms,
    };

    println!(
        "Built {} pages ({} written, {} drafts skipped) in {}ms",
        report.total_pages, report.pages_written, report.pages_skipped, report.elapsed_ms
    );

    Ok(report)
}

fn write_output(pages: &[Page], output_dir: &Path) -> Result<usize> {
    let mut written = 0;

    for page in pages {
        let html = match &page.rendered_html {
            Some(h) => h,
            None => continue,
        };

        // Clean URL: slug "blog/post" → output_dir/blog/post/index.html
        let dest = output_dir.join(&page.slug).join("index.html");
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, html)?;
        written += 1;
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SiteConfig;
    use std::path::PathBuf;

    fn noop_render(pages: &mut [Page]) {
        for page in pages {
            page.rendered_html = Some(page.raw_content.clone());
        }
    }

    fn test_config() -> SiteConfig {
        SiteConfig {
            title: "Test".to_string(),
            base_url: "http://localhost".to_string(),
            content_dir: PathBuf::from("content"),
            output_dir: PathBuf::from("public"),
            template_dir: PathBuf::from("templates"),
            data_dir: PathBuf::from("_data"),
        }
    }

    #[test]
    fn build_fixture_site() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("hello.md"),
            "---\ntitle: Hello\n---\n# Hello World",
        )
        .unwrap();

        let report = build(
            &config,
            dir.path(),
            false,
            noop_render,
            None::<fn(&Page, &SiteConfig) -> Result<String>>,
        )
        .unwrap();
        assert_eq!(report.total_pages, 1);
        assert_eq!(report.pages_written, 1);

        let output = dir.path().join("public/hello/index.html");
        assert!(output.exists());
    }

    #[test]
    fn drafts_are_skipped() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("draft.md"),
            "---\ntitle: Draft\ndraft: true\n---\nDraft content",
        )
        .unwrap();
        std::fs::write(
            content.join("published.md"),
            "---\ntitle: Published\n---\nPublished content",
        )
        .unwrap();

        let report = build(
            &config,
            dir.path(),
            false,
            noop_render,
            None::<fn(&Page, &SiteConfig) -> Result<String>>,
        )
        .unwrap();
        assert_eq!(report.pages_written, 1);
        assert_eq!(report.pages_skipped, 1);
    }
}
