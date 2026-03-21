//! Build pipeline orchestration with optional profiling.

use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;
use std::time::Instant;

use crate::cache::DepGraph;
use crate::config::SiteConfig;
use crate::content::discover_content;
use crate::page::Page;

/// Summary of a completed build.
#[derive(Debug)]
pub struct BuildReport {
    pub total_pages: usize,
    pub pages_written: usize,
    pub pages_unchanged: usize,
    pub pages_skipped: usize,
    pub elapsed_ms: u128,
    pub profile: Option<BuildProfile>,
}

/// Per-stage timing breakdown.
#[derive(Debug)]
pub struct BuildProfile {
    pub discovery_ms: u128,
    pub render_ms: u128,
    pub template_ms: u128,
    pub output_ms: u128,
}

impl BuildProfile {
    pub fn print(&self) {
        println!("\n  Pipeline profile:");
        println!("    Discovery:  {:>6}ms", self.discovery_ms);
        println!("    Render:     {:>6}ms", self.render_ms);
        println!("    Templates:  {:>6}ms", self.template_ms);
        println!("    Output:     {:>6}ms", self.output_ms);
    }
}

/// Run the full build pipeline with pluggable render and template steps.
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
    build_with_profile(config, root, include_drafts, render_fn, template_fn, false)
}

/// Run the build pipeline with optional profiling.
pub fn build_with_profile<R, T>(
    config: &SiteConfig,
    root: &Path,
    include_drafts: bool,
    render_fn: R,
    template_fn: Option<T>,
    profile: bool,
) -> Result<BuildReport>
where
    R: Fn(&mut [Page]),
    T: Fn(&Page, &SiteConfig) -> Result<String>,
{
    let start = Instant::now();

    // Discovery
    let t0 = Instant::now();
    let mut pages = discover_content(config, root)?;
    let discovery_ms = t0.elapsed().as_millis();

    // Filter drafts
    let pages_skipped = if !include_drafts {
        let before = pages.len();
        pages.retain(|p| !p.frontmatter.draft.unwrap_or(false));
        before - pages.len()
    } else {
        0
    };

    let total_pages = pages.len();

    // Load incremental cache
    let output_dir = root.join(&config.output_dir);
    let mut cache = DepGraph::load(&output_dir);

    // Render markdown
    let t1 = Instant::now();
    render_fn(&mut pages);
    let render_ms = t1.elapsed().as_millis();

    // Apply templates if provided
    let t2 = Instant::now();
    if let Some(ref tmpl_fn) = template_fn {
        for page in &mut pages {
            let html = tmpl_fn(page, config)?;
            page.rendered_html = Some(html);
        }
    }
    let template_ms = t2.elapsed().as_millis();

    // Write output files (incremental, parallelized)
    let t3 = Instant::now();

    // Separate changed from unchanged
    let mut to_write: Vec<&Page> = Vec::new();
    let mut pages_unchanged = 0;

    for page in &pages {
        if page.rendered_html.is_none() {
            continue;
        }
        if !cache.is_changed(&page.slug, page.content_hash) {
            pages_unchanged += 1;
        } else {
            to_write.push(page);
        }
    }

    // Parallel file writes
    let write_errors: Vec<_> = to_write
        .par_iter()
        .filter_map(|page| {
            let html = page.rendered_html.as_ref().unwrap();
            let dest = output_dir.join(&page.slug).join("index.html");
            if let Some(parent) = dest.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return Some(e.into());
                }
            }
            if let Err(e) = std::fs::write(&dest, html) {
                return Some(e.into());
            }
            None
        })
        .collect::<Vec<anyhow::Error>>();

    if let Some(err) = write_errors.into_iter().next() {
        return Err(err);
    }

    let pages_written = to_write.len();

    // Update cache for written pages
    for page in &to_write {
        cache.record(&page.slug, page.content_hash);
    }
    cache.save()?;

    let output_ms = t3.elapsed().as_millis();
    let elapsed_ms = start.elapsed().as_millis();

    let build_profile = if profile {
        Some(BuildProfile {
            discovery_ms,
            render_ms,
            template_ms,
            output_ms,
        })
    } else {
        None
    };

    let report = BuildReport {
        total_pages,
        pages_written,
        pages_unchanged,
        pages_skipped,
        elapsed_ms,
        profile: build_profile,
    };

    println!(
        "Built {} pages ({} written, {} unchanged, {} drafts skipped) in {}ms",
        report.total_pages,
        report.pages_written,
        report.pages_unchanged,
        report.pages_skipped,
        report.elapsed_ms
    );

    if let Some(ref prof) = report.profile {
        prof.print();
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SiteConfig;

    fn noop_render(pages: &mut [Page]) {
        for page in pages {
            page.rendered_html = Some(page.raw_content.clone());
        }
    }

    fn test_config() -> SiteConfig {
        SiteConfig::for_testing("Test", "http://localhost")
    }

    type NoTemplate = fn(&Page, &SiteConfig) -> Result<String>;

    fn do_build(config: &SiteConfig, root: &Path) -> BuildReport {
        build(config, root, false, noop_render, None::<NoTemplate>).unwrap()
    }

    #[test]
    fn full_build_writes_all_and_creates_cache() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nPage A").unwrap();
        std::fs::write(content.join("b.md"), "---\ntitle: B\n---\nPage B").unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.total_pages, 2);
        assert_eq!(report.pages_written, 2);
        assert_eq!(report.pages_unchanged, 0);

        let cache_path = dir.path().join("public/.mythic-cache.json");
        assert!(cache_path.exists());
    }

    #[test]
    fn noop_rebuild_writes_zero() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nPage A").unwrap();
        std::fs::write(content.join("b.md"), "---\ntitle: B\n---\nPage B").unwrap();

        do_build(&config, dir.path());

        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_written, 0);
        assert_eq!(report.pages_unchanged, 2);
    }

    #[test]
    fn single_file_changed_rebuilds_only_that() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nPage A").unwrap();
        std::fs::write(content.join("b.md"), "---\ntitle: B\n---\nPage B").unwrap();

        do_build(&config, dir.path());
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nPage A updated").unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_written, 1);
        assert_eq!(report.pages_unchanged, 1);
    }

    #[test]
    fn deleted_cache_forces_full_rebuild() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nPage A").unwrap();

        do_build(&config, dir.path());
        std::fs::remove_file(dir.path().join("public/.mythic-cache.json")).unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_written, 1);
        assert_eq!(report.pages_unchanged, 0);
    }

    #[test]
    fn drafts_are_skipped() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("draft.md"), "---\ntitle: Draft\ndraft: true\n---\nDraft").unwrap();
        std::fs::write(content.join("pub.md"), "---\ntitle: Pub\n---\nPublished").unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_written, 1);
        assert_eq!(report.pages_skipped, 1);
    }

    #[test]
    fn profile_flag_produces_timing() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nPage A").unwrap();

        let report = build_with_profile(
            &config, dir.path(), false, noop_render, None::<NoTemplate>, true,
        ).unwrap();

        assert!(report.profile.is_some());
        let prof = report.profile.unwrap();
        assert!(prof.discovery_ms + prof.render_ms + prof.template_ms + prof.output_ms <= report.elapsed_ms + 1);
    }
}
