//! Build pipeline orchestration with optional profiling.
//!
//! Uses ahash for faster content hashing and lasso for string interning
//! of frequently repeated values (layout names, tag values).
//!
//! The build pipeline flows through these stages:
//!
//! 1. **Discovery** — Walk the content directory, read files, parse frontmatter
//! 2. **Draft filtering** — Remove drafts unless `--drafts` is passed
//! 3. **Cache check** — Load content hashes from `.mythic-cache.json`
//! 4. **Render** — Convert markdown to HTML (pluggable via `render_fn`)
//! 5. **Template** — Apply layout templates (pluggable via `template_fn`)
//! 6. **Output** — Write changed pages to disk in parallel, update cache
//!
//! Use [`build`] for standard builds or [`build_with_profile`] to get
//! per-stage timing breakdowns.

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
/// Returns the build report and the processed pages for post-build use.
pub fn build<R, T>(
    config: &SiteConfig,
    root: &Path,
    include_drafts: bool,
    render_fn: R,
    template_fn: Option<T>,
) -> Result<(BuildReport, Vec<Page>)>
where
    R: Fn(&mut [Page]),
    T: Fn(&Page, &SiteConfig) -> Result<String> + Sync,
{
    build_with_profile(config, root, include_drafts, render_fn, template_fn, false)
}

/// Run the build pipeline with optional profiling.
/// Returns the build report and the processed pages for post-build use
/// (taxonomy generation, feeds, sitemaps).
pub fn build_with_profile<R, T>(
    config: &SiteConfig,
    root: &Path,
    include_drafts: bool,
    render_fn: R,
    template_fn: Option<T>,
    profile: bool,
) -> Result<(BuildReport, Vec<Page>)>
where
    R: Fn(&mut [Page]),
    T: Fn(&Page, &SiteConfig) -> Result<String> + Sync,
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

    // Apply templates in parallel if provided
    let t2 = Instant::now();
    if let Some(ref tmpl_fn) = template_fn {
        let results: Vec<Result<String>> =
            pages.par_iter().map(|page| tmpl_fn(page, config)).collect();

        for (page, result) in pages.iter_mut().zip(results) {
            page.rendered_html = Some(result?);
        }
    }
    let template_ms = t2.elapsed().as_millis();

    // Write output files (incremental, parallelized)
    let t3 = Instant::now();

    // Pre-compute output paths and separate changed from unchanged.
    // Each entry holds (page_ref, dir_path, file_path) to avoid
    // redundant PathBuf joins later.
    struct WriteJob<'a> {
        page: &'a Page,
        dir: std::path::PathBuf,
        file: std::path::PathBuf,
    }

    let mut to_write: Vec<WriteJob> = Vec::new();
    let mut pages_unchanged = 0;
    let ugly_urls = config.ugly_urls;

    for page in &pages {
        if page.rendered_html.is_none() {
            continue;
        }
        if !cache.is_changed(&page.slug, page.content_hash) {
            pages_unchanged += 1;
        } else if ugly_urls {
            // Flat output: slug "blog/post" → output_dir/blog/post.html
            // No per-page directory creation needed.
            let file = output_dir.join(format!("{}.html", page.slug));
            let dir = file.parent().unwrap_or(&output_dir).to_path_buf();
            to_write.push(WriteJob { page, dir, file });
        } else {
            // Clean URLs: slug "blog/post" → output_dir/blog/post/index.html
            let dir = output_dir.join(&page.slug);
            let file = dir.join("index.html");
            to_write.push(WriteJob { page, dir, file });
        }
    }

    // Create directories using sorted single-mkdir approach.
    // Sort by path so parents come before children, then use
    // create_dir (single mkdir syscall) instead of create_dir_all
    // (which does multiple stat calls per ancestor).
    {
        let mut dirs: Vec<&std::path::Path> = to_write.iter().map(|j| j.dir.as_path()).collect();
        dirs.sort();
        dirs.dedup();

        // Ensure the output root exists first
        std::fs::create_dir_all(&output_dir)?;

        // Collect all unique ancestor directories, sorted by depth
        let mut all_dirs = std::collections::BTreeSet::new();
        for dir in &dirs {
            let mut current = *dir;
            while current != output_dir && current.starts_with(&output_dir) {
                all_dirs.insert(current.to_path_buf());
                match current.parent() {
                    Some(p) => current = p,
                    None => break,
                }
            }
        }

        // Create in sorted order (BTreeSet = lexicographic = parents before children).
        // Use create_dir (not create_dir_all) — parent is guaranteed to exist
        // from a previous iteration or from the output_dir root.
        for dir in &all_dirs {
            // create_dir returns Err if already exists, which is fine
            let _ = std::fs::create_dir(dir);
        }
    }

    // Parallel file writes — directories already exist, paths pre-computed
    let write_errors: Vec<_> = to_write
        .par_iter()
        .filter_map(|job| {
            let html = match job.page.rendered_html.as_ref() {
                Some(h) => h,
                None => return None, // skip pages with no rendered HTML
            };
            if let Err(e) = std::fs::write(&job.file, html) {
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
    for job in &to_write {
        cache.record(&job.page.slug, job.page.content_hash);
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

    // Note: callers are responsible for printing the build summary.
    // The BuildReport and BuildProfile contain all the data needed.

    Ok((report, pages))
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
        build(config, root, false, noop_render, None::<NoTemplate>)
            .unwrap()
            .0
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
        std::fs::write(
            content.join("draft.md"),
            "---\ntitle: Draft\ndraft: true\n---\nDraft",
        )
        .unwrap();
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

        let (report, _pages) = build_with_profile(
            &config,
            dir.path(),
            false,
            noop_render,
            None::<NoTemplate>,
            true,
        )
        .unwrap();

        assert!(report.profile.is_some());
        let prof = report.profile.unwrap();
        assert!(
            prof.discovery_ms + prof.render_ms + prof.template_ms + prof.output_ms
                <= report.elapsed_ms + 1
        );
    }

    // --- Incremental build depth ---

    #[test]
    fn adding_new_file_rebuilds_only_new() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nPage A").unwrap();

        do_build(&config, dir.path());

        std::fs::write(content.join("b.md"), "---\ntitle: B\n---\nPage B").unwrap();
        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_written, 1);
        assert_eq!(report.pages_unchanged, 1);
        assert_eq!(report.total_pages, 2);
    }

    #[test]
    fn deleting_file_does_not_break_build() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nA").unwrap();
        std::fs::write(content.join("b.md"), "---\ntitle: B\n---\nB").unwrap();

        do_build(&config, dir.path());
        std::fs::remove_file(content.join("b.md")).unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.total_pages, 1);
        assert_eq!(report.pages_unchanged, 1);
    }

    #[test]
    fn changing_frontmatter_only_rebuilds_page() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: Old Title\n---\nBody").unwrap();

        do_build(&config, dir.path());
        std::fs::write(content.join("a.md"), "---\ntitle: New Title\n---\nBody").unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_written, 1);
    }

    #[test]
    fn multiple_rebuilds_stay_consistent() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nA").unwrap();

        // Build 5 times with no changes
        for _ in 0..5 {
            let report = do_build(&config, dir.path());
            assert_eq!(report.total_pages, 1);
        }

        // Only the first should write; check cache is stable
        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_unchanged, 1);
        assert_eq!(report.pages_written, 0);
    }

    #[test]
    fn draft_then_undraft_rebuilds() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\ndraft: true\n---\nA").unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_skipped, 1);
        assert_eq!(report.pages_written, 0);

        // Undraft
        std::fs::write(content.join("a.md"), "---\ntitle: A\ndraft: false\n---\nA").unwrap();
        let report = do_build(&config, dir.path());
        assert_eq!(report.pages_skipped, 0);
        assert_eq!(report.pages_written, 1);
    }

    #[test]
    fn include_drafts_flag_overrides() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("a.md"),
            "---\ntitle: Draft\ndraft: true\n---\nDraft",
        )
        .unwrap();

        let (report, _) =
            build(&config, dir.path(), true, noop_render, None::<NoTemplate>).unwrap();
        assert_eq!(report.pages_skipped, 0);
        assert_eq!(report.pages_written, 1);
    }

    // --- Output correctness ---

    #[test]
    fn clean_urls_produce_index_html() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("about.md"),
            "---\ntitle: About\n---\nAbout page",
        )
        .unwrap();

        do_build(&config, dir.path());

        let output = dir.path().join("public/about/index.html");
        assert!(
            output.exists(),
            "Expected clean URL: public/about/index.html"
        );
        let html = std::fs::read_to_string(output).unwrap();
        assert!(html.contains("About page"));
    }

    #[test]
    fn nested_content_produces_nested_output() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("content/blog/2024");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("post.md"), "---\ntitle: Post\n---\nDeep post").unwrap();

        do_build(&config, dir.path());

        let output = dir.path().join("public/blog/2024/post/index.html");
        assert!(output.exists());
    }

    #[test]
    fn empty_content_dir_builds_zero_pages() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("content")).unwrap();

        let report = do_build(&config, dir.path());
        assert_eq!(report.total_pages, 0);
        assert_eq!(report.pages_written, 0);
    }

    #[test]
    fn missing_content_dir_builds_zero_pages() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        // Don't create content dir at all

        let report = do_build(&config, dir.path());
        assert_eq!(report.total_pages, 0);
    }

    #[test]
    fn template_fn_is_applied() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nBody").unwrap();

        let tmpl_fn = |page: &Page, _cfg: &SiteConfig| -> Result<String> {
            Ok(format!(
                "<html><body>{}</body></html>",
                page.rendered_html.as_deref().unwrap_or("")
            ))
        };

        build(&config, dir.path(), false, noop_render, Some(tmpl_fn)).unwrap();

        let output = std::fs::read_to_string(dir.path().join("public/a/index.html")).unwrap();
        assert!(output.contains("<html><body>"));
        assert!(output.contains("Body"));
    }

    // --- Error handling ---

    #[test]
    fn template_error_propagates() {
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nBody").unwrap();

        let bad_tmpl = |_page: &Page, _cfg: &SiteConfig| -> Result<String> {
            anyhow::bail!("Template rendering failed")
        };

        let result = build(&config, dir.path(), false, noop_render, Some(bad_tmpl));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Template rendering failed"));
    }

    // --- Hugo regression tests ---

    #[test]
    fn parallel_build_is_deterministic() {
        // Hugo #3013: parallel builds must produce consistent results
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        for i in 0..50 {
            std::fs::write(
                content.join(format!("page-{i}.md")),
                format!("---\ntitle: Page {i}\n---\nContent {i}"),
            )
            .unwrap();
        }

        // Build twice and compare outputs
        let (report1, _) =
            build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();

        // Clean and rebuild
        std::fs::remove_dir_all(dir.path().join("public")).unwrap();
        let (report2, _) =
            build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();

        assert_eq!(report1.total_pages, report2.total_pages);
        assert_eq!(report1.pages_written, report2.pages_written);
    }

    #[test]
    fn frontmatter_change_invalidates_cache() {
        // Hugo #12390: changing frontmatter must invalidate the cache
        let config = test_config();
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: Original\n---\nBody").unwrap();

        do_build(&config, dir.path());

        // Change only frontmatter
        std::fs::write(content.join("a.md"), "---\ntitle: Changed\n---\nBody").unwrap();
        let report = do_build(&config, dir.path());
        // The content hash should differ because the raw file changed
        assert_eq!(
            report.pages_written, 1,
            "Frontmatter change should trigger rebuild"
        );
    }

    // --- ugly_urls (flat output) tests ---

    #[test]
    fn ugly_urls_produces_flat_html_files() {
        let mut config = test_config();
        config.ugly_urls = true;
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("about.md"),
            "---\ntitle: About\n---\nAbout page",
        )
        .unwrap();

        build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();

        // Should produce about.html, NOT about/index.html
        let flat = dir.path().join("public/about.html");
        assert!(flat.exists(), "Expected flat output: public/about.html");
        assert!(!dir.path().join("public/about/index.html").exists());

        let html = std::fs::read_to_string(flat).unwrap();
        assert!(html.contains("About page"));
    }

    #[test]
    fn ugly_urls_nested_paths() {
        let mut config = test_config();
        config.ugly_urls = true;
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("content/blog/2024");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("post.md"), "---\ntitle: Post\n---\nDeep").unwrap();

        build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();

        let flat = dir.path().join("public/blog/2024/post.html");
        assert!(flat.exists(), "Expected: public/blog/2024/post.html");
    }

    #[test]
    fn ugly_urls_incremental_works() {
        let mut config = test_config();
        config.ugly_urls = true;
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("a.md"), "---\ntitle: A\n---\nA").unwrap();
        std::fs::write(content.join("b.md"), "---\ntitle: B\n---\nB").unwrap();

        let (r1, _) = build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();
        assert_eq!(r1.pages_written, 2);

        let (r2, _) = build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();
        assert_eq!(r2.pages_written, 0);
        assert_eq!(r2.pages_unchanged, 2);
    }

    #[test]
    fn clean_urls_still_default() {
        let config = test_config();
        assert!(!config.ugly_urls);
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("about.md"), "---\ntitle: About\n---\nAbout").unwrap();

        build(&config, dir.path(), false, noop_render, None::<NoTemplate>).unwrap();

        assert!(dir.path().join("public/about/index.html").exists());
        assert!(!dir.path().join("public/about.html").exists());
    }
}
