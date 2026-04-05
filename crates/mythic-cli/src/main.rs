//! CLI binary for the Mythic static site generator.

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use owo_colors::OwoColorize;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "mythic",
    version,
    about = "A fast static site generator written in Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Suppress output (for CI/scripting)
    #[arg(short, long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the site
    Build {
        /// Path to config file
        #[arg(short, long, default_value = "mythic.toml")]
        config: PathBuf,
        /// Include draft pages
        #[arg(long)]
        drafts: bool,
        /// Delete output directory before building
        #[arg(long)]
        clean: bool,
        /// Print per-stage timing breakdown
        #[arg(long)]
        profile: bool,
        /// Output build report as JSON (for CI)
        #[arg(long)]
        json: bool,
    },
    /// Start the development server with live reload
    Serve {
        /// Path to config file
        #[arg(short, long, default_value = "mythic.toml")]
        config: PathBuf,
        /// Port to listen on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
        /// Include draft pages
        #[arg(long)]
        drafts: bool,
        /// Open browser automatically
        #[arg(long)]
        open: bool,
    },
    /// Create a new Mythic site
    Init {
        /// Project name
        name: String,
        /// Starter template: blank, blog, docs, portfolio
        #[arg(short, long, default_value = "blank")]
        template: String,
    },
    /// Create a new content file with frontmatter scaffold
    New {
        /// Content type (e.g., "post", "page", "doc")
        content_type: String,
        /// Title of the new content
        title: String,
        /// Path to config file
        #[arg(short, long, default_value = "mythic.toml")]
        config: PathBuf,
        /// Create as draft
        #[arg(long)]
        draft: bool,
    },
    /// Check the built site for broken links and issues
    Check {
        /// Path to config file
        #[arg(short, long, default_value = "mythic.toml")]
        config: PathBuf,
    },
    /// Migrate from another static site generator
    Migrate {
        /// Source SSG: jekyll, hugo, or eleventy
        #[arg(long)]
        from: String,
        /// Path to the source project
        #[arg(long)]
        source: PathBuf,
        /// Output path for the migrated Mythic project
        #[arg(long)]
        output: PathBuf,
    },
    /// Delete the output directory
    Clean {
        /// Path to config file
        #[arg(short, long, default_value = "mythic.toml")]
        config: PathBuf,
    },
    /// List all content pages
    List {
        /// Path to config file
        #[arg(short, long, default_value = "mythic.toml")]
        config: PathBuf,
        /// Include draft pages
        #[arg(long)]
        drafts: bool,
    },
    /// Watch for changes and rebuild (without starting a server)
    Watch {
        /// Path to config file
        #[arg(short, long, default_value = "mythic.toml")]
        config: PathBuf,
        /// Include draft pages
        #[arg(long)]
        drafts: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let quiet = cli.quiet;

    match cli.command {
        Commands::Build {
            config,
            drafts,
            clean,
            profile,
            json,
        } => {
            let site_config = load_config_with_validation(&config, quiet)?;
            let root = config.parent().unwrap_or_else(|| Path::new("."));

            if clean {
                let output = root.join(&site_config.output_dir);
                if output.exists() {
                    std::fs::remove_dir_all(&output)?;
                }
            }

            full_build(&site_config, root, drafts, profile, quiet, json)?;
        }
        Commands::Serve {
            config,
            port,
            drafts,
            open,
        } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(cmd_serve(&config, port, drafts, open))?;
        }
        Commands::Init { name, template } => {
            init_project(&name, &template)?;
        }
        Commands::New {
            content_type,
            title,
            config,
            draft,
        } => {
            cmd_new(&config, &content_type, &title, draft)?;
        }
        Commands::Check { config } => {
            let site_config = mythic_core::config::load_config(&config)?;
            let root = config.parent().unwrap_or_else(|| Path::new("."));
            let output_dir = root.join(&site_config.output_dir);

            let report = mythic_core::check::check_site(&output_dir)?;
            print_check_report(&report);

            if report.has_errors() {
                std::process::exit(1);
            }
        }
        Commands::Migrate {
            from,
            source,
            output,
        } => {
            let report = match from.as_str() {
                "jekyll" => mythic_core::migrate::jekyll::migrate(&source, &output)?,
                "hugo" => mythic_core::migrate::hugo::migrate(&source, &output)?,
                "hugo-theme" => mythic_core::migrate::hugo_theme::convert_theme(&source, &output)?,
                "eleventy" | "11ty" => mythic_core::migrate::eleventy::migrate(&source, &output)?,
                other => {
                    anyhow::bail!(
                        "Unknown source: {other}. Supported: jekyll, hugo, hugo-theme, eleventy"
                    )
                }
            };
            if !quiet {
                print_migration_report(&report);
            }
        }
        Commands::Clean { config } => {
            let site_config = mythic_core::config::load_config(&config)?;
            let root = config.parent().unwrap_or_else(|| Path::new("."));
            let output = root.join(&site_config.output_dir);
            if output.exists() {
                std::fs::remove_dir_all(&output)?;
                if !quiet {
                    println!("{} {}", "Cleaned".green().bold(), output.display());
                }
            } else if !quiet {
                println!("  {} output directory does not exist", "note:".dimmed());
            }
        }
        Commands::List { config, drafts } => {
            let site_config = mythic_core::config::load_config(&config)?;
            let root = config.parent().unwrap_or_else(|| Path::new("."));
            let mut pages = mythic_core::content::discover_content(&site_config, root)?;
            pages.sort_by(|a, b| a.slug.cmp(&b.slug));

            for page in &pages {
                let is_draft = page.frontmatter.draft.unwrap_or(false);
                if is_draft && !drafts {
                    continue;
                }
                let date = page.frontmatter.date.as_deref().unwrap_or("          ");
                let draft_marker = if is_draft {
                    format!(" {}", "[draft]".yellow())
                } else {
                    String::new()
                };
                println!(
                    "  {} {} {}{}",
                    date.dimmed(),
                    page.slug.bold(),
                    page.frontmatter.title.dimmed(),
                    draft_marker,
                );
            }
            println!(
                "\n  {} pages{}",
                pages
                    .iter()
                    .filter(|p| drafts || !p.frontmatter.draft.unwrap_or(false))
                    .count(),
                if drafts { " (including drafts)" } else { "" },
            );
        }
        Commands::Watch { config, drafts } => {
            let site_config = load_config_with_validation(&config, quiet)?;
            let root = config
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();

            full_build(&site_config, &root, drafts, false, quiet, false)?;

            let watcher = mythic_server::watcher::FileWatcher::new(&site_config, &root)?;
            if !quiet {
                println!("  {} for changes...", "Watching".cyan());
            }

            while let Ok(event) = watcher.rx.recv() {
                if !quiet {
                    println!("  {} {event:?}", "Change detected:".cyan());
                }
                if let Err(e) = full_build(&site_config, &root, drafts, false, quiet, false) {
                    eprintln!("  {} {e}", "Build error:".red().bold());
                }
            }
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "mythic", &mut std::io::stdout());
        }
    }

    Ok(())
}

// --- Config loading with validation ---

fn load_config_with_validation(
    path: &Path,
    quiet: bool,
) -> Result<mythic_core::config::SiteConfig> {
    let config = mythic_core::config::load_config(path)?;

    // Validate: warn on common issues
    if !quiet {
        if config.base_url.is_empty() {
            eprintln!(
                "  {} base_url is empty in {}",
                "warning:".yellow().bold(),
                path.display()
            );
        }
        if config.base_url.ends_with('/') && config.base_url != "/" {
            eprintln!(
                "  {} base_url has trailing slash (may cause double slashes in URLs)",
                "warning:".yellow().bold(),
            );
        }
    }

    // Check for unrecognized keys by re-parsing as a generic table
    let raw = std::fs::read_to_string(path)?;
    if let Ok(table) = raw.parse::<toml::Table>() {
        let known_keys = [
            "title",
            "base_url",
            "content_dir",
            "output_dir",
            "template_dir",
            "data_dir",
            "static_dir",
            "styles_dir",
            "scripts_dir",
            "image_breakpoints",
            "sass",
            "taxonomies",
            "feed",
            "highlight",
            "toc",
            "sitemap",
            "templates",
            "i18n",
            "ugly_urls",
            "remote",
            "lint",
        ];
        if !quiet {
            for key in table.keys() {
                if !known_keys.contains(&key.as_str()) {
                    eprintln!(
                        "  {} unrecognized config key '{}' in {}",
                        "warning:".yellow().bold(),
                        key.yellow(),
                        path.display()
                    );
                }
            }
        }
    }

    Ok(config)
}

// --- Colored output helpers ---

fn print_build_summary(report: &mythic_core::build::BuildReport) {
    let status = if report.pages_written > 0 {
        "Built".green().bold().to_string()
    } else {
        "Built".cyan().bold().to_string()
    };

    println!(
        "{} {} pages ({} written, {} unchanged, {} drafts skipped) in {}",
        status,
        report.total_pages.to_string().bold(),
        report.pages_written.to_string().green(),
        report.pages_unchanged.to_string().dimmed(),
        report.pages_skipped.to_string().dimmed(),
        format!("{}ms", report.elapsed_ms).yellow(),
    );

    if let Some(ref prof) = report.profile {
        println!("\n  {} ", "Pipeline profile:".dimmed());
        println!("    Discovery:  {:>6}ms", prof.discovery_ms);
        println!("    Render:     {:>6}ms", prof.render_ms);
        println!("    Templates:  {:>6}ms", prof.template_ms);
        println!("    Output:     {:>6}ms", prof.output_ms);
    }
}

fn print_check_report(report: &mythic_core::check::CheckReport) {
    println!("\n{} results:", "Check".cyan().bold());
    println!(
        "  Pages checked: {}",
        report.pages_checked.to_string().bold()
    );
    println!(
        "  Links checked: {}",
        report.links_checked.to_string().bold()
    );

    if !report.errors.is_empty() {
        println!("\n  {} ({}):", "Errors".red().bold(), report.errors.len());
        for e in &report.errors {
            println!("    {} {} {}", "x".red(), e.file.dimmed(), e.message);
        }
    }

    if !report.warnings.is_empty() {
        println!(
            "\n  {} ({}):",
            "Warnings".yellow().bold(),
            report.warnings.len()
        );
        for w in &report.warnings {
            println!("    {} {} {}", "!".yellow(), w.file.dimmed(), w.message);
        }
    }

    if report.errors.is_empty() && report.warnings.is_empty() {
        println!("  {}", "No issues found.".green());
    }
}

fn print_migration_report(report: &mythic_core::migrate::MigrationReport) {
    println!("\n{}", "Migration complete:".green().bold());
    println!("  Files copied:    {}", report.files_copied);
    println!("  Files converted: {}", report.files_converted);

    if !report.warnings.is_empty() {
        println!(
            "\n  {} ({}):",
            "Warnings".yellow().bold(),
            report.warnings.len()
        );
        for w in &report.warnings {
            println!("    {} {w}", "!".yellow());
        }
    }

    if !report.errors.is_empty() {
        println!("\n  {} ({}):", "Errors".red().bold(), report.errors.len());
        for e in &report.errors {
            println!("    {} {e}", "x".red());
        }
    }
}

fn format_template_error(err: &anyhow::Error) -> String {
    let msg = err.to_string();

    // Extract useful info from Tera errors
    if msg.contains("Failed to render") {
        if let Some(cause) = msg.split("Caused by:").nth(1) {
            let cause = cause.trim();
            // Extract variable name from "Variable `X` not found"
            if cause.contains("not found in context") {
                return format!(
                    "{} {cause}\n    {} Check your template variables match the context (page.*, site.*, content, toc, assets.*, data.*)",
                    "Template error:".red().bold(),
                    "hint:".cyan(),
                );
            }
            return format!("{} {cause}", "Template error:".red().bold());
        }
    }

    // Extract useful info from Handlebars errors
    if msg.contains("Failed to render Handlebars") {
        return format!("{} {msg}", "Template error:".red().bold());
    }

    msg
}

// --- mythic new command ---

fn cmd_new(config_path: &Path, content_type: &str, title: &str, draft: bool) -> Result<()> {
    let site_config = mythic_core::config::load_config(config_path)?;
    let root = config_path.parent().unwrap_or_else(|| Path::new("."));

    // Slugify title for filename
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // Determine output path based on content type
    let content_dir = root.join(&site_config.content_dir);
    let dir = if content_type == "page" {
        content_dir.clone()
    } else {
        content_dir.join(format!("{content_type}s"))
    };

    std::fs::create_dir_all(&dir)?;
    let file_path = dir.join(format!("{slug}.md"));

    if file_path.exists() {
        anyhow::bail!("File already exists: {}", file_path.display());
    }

    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let draft_line = if draft { "\ndraft: true" } else { "" };

    let content = format!("---\ntitle: \"{title}\"\ndate: \"{date}\"{draft_line}\n---\n\n");

    std::fs::write(&file_path, content)?;

    let rel = file_path.strip_prefix(root).unwrap_or(&file_path);

    println!("{} {}", "Created".green().bold(), rel.display());
    if draft {
        println!("  (marked as {})", "draft".yellow());
    }

    Ok(())
}

// --- Full build pipeline ---

fn full_build(
    site_config: &mythic_core::config::SiteConfig,
    root: &Path,
    drafts: bool,
    profile: bool,
    quiet: bool,
    json: bool,
) -> Result<()> {
    let output_dir = root.join(&site_config.output_dir);
    let full_start = std::time::Instant::now();

    // --- Pre-build: load data, plugins ---

    let data_dir = root.join(&site_config.data_dir);
    let mut site_data = mythic_core::data::load_data(&data_dir)?;

    // Fetch remote data sources
    if !site_config.remote.is_empty() {
        let (remote_data, remote_warnings) =
            mythic_core::remote::fetch_remote_data(&site_config.remote, &data_dir)?;
        for w in &remote_warnings {
            eprintln!("  {} {w}", "warning:".yellow().bold());
        }
        if let serde_json::Value::Object(ref mut map) = site_data {
            map.insert("remote".to_string(), remote_data);
        }
    }

    // Pre-build content discovery for collections.
    // Collections are registered as lazy Tera functions (get_pages, get_sections)
    // instead of being included in the per-page template context. This avoids
    // deep-cloning the entire page list for every page render (O(n²) → O(1) for
    // templates that don't use collections, which is the common case).
    let collections_data = {
        let pre_pages = mythic_core::content::discover_content(site_config, root)?;

        let all_pages_json: Vec<serde_json::Value> = pre_pages
            .iter()
            .filter(|p| !p.frontmatter.draft.unwrap_or(false) || drafts)
            .map(|p| {
                serde_json::json!({
                    "title": p.frontmatter.title.as_str(),
                    "slug": &p.slug,
                    "url": format!("/{}/", p.slug),
                    "date": p.frontmatter.date.as_deref(),
                    "tags": p.frontmatter.tags.as_ref().map(|t| t.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
                })
            })
            .collect();

        let mut sections: std::collections::HashMap<String, Vec<serde_json::Value>> =
            std::collections::HashMap::new();
        for p in &pre_pages {
            if p.frontmatter.draft.unwrap_or(false) && !drafts {
                continue;
            }
            let section = p.slug.split('/').next().unwrap_or("").to_string();
            if !section.is_empty() && section != p.slug {
                sections
                    .entry(section)
                    .or_default()
                    .push(serde_json::json!({
                        "title": p.frontmatter.title.as_str(),
                        "slug": &p.slug,
                        "url": format!("/{}/", p.slug),
                        "date": p.frontmatter.date.as_deref(),
                    }));
            }
        }

        let mut collections = serde_json::Map::new();
        collections.insert(
            "pages".to_string(),
            serde_json::Value::Array(all_pages_json),
        );
        collections.insert(
            "sections".to_string(),
            serde_json::to_value(&sections).unwrap_or_default(),
        );
        serde_json::Value::Object(collections)
    };

    let mut plugin_manager = mythic_core::plugin::PluginManager::new();
    plugin_manager.register(Box::new(mythic_core::plugin::ReadingTimePlugin::new()));

    let plugins_dir = root.join("plugins");
    if plugins_dir.exists() {
        let rhai_plugins = mythic_core::rhai_plugin::load_rhai_plugins(&plugins_dir)?;
        for plugin in rhai_plugins {
            plugin_manager.register(plugin);
        }
    }

    plugin_manager.run_pre_build(site_config)?;

    let template_dir = root.join(&site_config.template_dir);
    let default_engine = site_config
        .templates
        .as_ref()
        .map(|t| t.default_engine.as_str())
        .unwrap_or("tera");
    let mut engine =
        mythic_template::TemplateEngine::new_with_default(&template_dir, default_engine)?;

    // Register collections as lazy Tera functions — only materialized when called.
    // This eliminates the O(n²) clone overhead for templates that don't use collections.
    if let serde_json::Value::Object(ref coll) = collections_data {
        if let Some(pages_val) = coll.get("pages") {
            engine.register_lazy_value("get_pages", pages_val.clone());
        }
        if let Some(sections_val) = coll.get("sections") {
            engine.register_lazy_value("get_sections", sections_val.clone());
        }
    }

    let render_config = mythic_markdown::render::RenderConfig {
        highlight_theme: site_config
            .highlight
            .as_ref()
            .map(|h| h.theme.clone())
            .unwrap_or_else(|| "base16-ocean.dark".to_string()),
        line_numbers: site_config
            .highlight
            .as_ref()
            .map(|h| h.line_numbers)
            .unwrap_or(false),
        toc_min_level: site_config.toc.as_ref().map(|t| t.min_level).unwrap_or(2),
        toc_max_level: site_config.toc.as_ref().map(|t| t.max_level).unwrap_or(4),
    };

    let shortcode_dir = root.join("shortcodes");
    let has_shortcodes = shortcode_dir.exists();

    let (assets_manifest, asset_warnings) = mythic_assets::process_assets(site_config, root)?;
    for w in &asset_warnings {
        eprintln!("  {} {w}", "warning:".yellow().bold());
    }
    let mut template_extra = serde_json::Map::new();
    template_extra.insert(
        "css_path".to_string(),
        serde_json::to_value(&assets_manifest.css_path)?,
    );
    template_extra.insert(
        "js_path".to_string(),
        serde_json::to_value(&assets_manifest.js_path)?,
    );
    let assets_value = serde_json::Value::Object(template_extra);

    let highlighter = mythic_markdown::highlight::Highlighter::new(
        &render_config.highlight_theme,
        render_config.line_numbers,
    );
    let highlight_css = highlighter.generate_css();
    if !highlight_css.is_empty() {
        std::fs::create_dir_all(&output_dir)?;
        std::fs::write(output_dir.join("syntax.css"), &highlight_css)?;
    }

    // Pre-build the shared template context once (avoids re-serializing large
    // collections data for every page render — critical for O(n) vs O(n²) scaling).
    let base_ctx =
        engine.build_base_tera_context(site_config, Some(&assets_value), Some(&site_data));

    // --- Core build ---

    let (report, built_pages) = mythic_core::build::build_with_profile(
        site_config,
        root,
        drafts,
        |pages| {
            let content_dir = root.join(&site_config.content_dir);
            if let Err(e) = mythic_core::cascade::apply_cascade(pages, &content_dir) {
                eprintln!("  {} {e}", "cascade error:".red());
            }

            if let Err(e) = plugin_manager.run_all_discovered(pages) {
                eprintln!("  {} {e}", "plugin error:".red());
            }

            // Extract content summaries (<!--more--> marker or auto-truncate)
            mythic_core::summary::extract_summaries(pages);

            // Evaluate computed frontmatter fields (rhai: expressions)
            let computed_warnings = mythic_core::computed::evaluate_computed_fields(pages);
            for w in &computed_warnings {
                eprintln!("  {} {w}", "warning:".yellow().bold());
            }

            for page in pages.iter_mut() {
                if let Err(e) = plugin_manager.run_pre_render(page) {
                    eprintln!("  {} {e}", "plugin error:".red());
                }

                if has_shortcodes {
                    match mythic_markdown::shortcodes::process_shortcodes(
                        &page.raw_content,
                        &shortcode_dir,
                    ) {
                        Ok(processed) => page.raw_content = processed,
                        Err(e) => eprintln!("  {} in {}: {e}", "shortcode error".red(), page.slug),
                    }
                }
            }

            if let Some(ref i18n_config) = site_config.i18n {
                mythic_core::i18n::process_i18n(pages, i18n_config);
            }

            mythic_markdown::render::render_markdown_with_config(pages, &render_config);

            for page in pages.iter_mut() {
                if let Err(e) = plugin_manager.run_post_render(page) {
                    eprintln!("  {} {e}", "plugin error:".red());
                }
            }
        },
        Some(
            |page: &mythic_core::page::Page, _cfg: &mythic_core::config::SiteConfig| {
                match engine.render_with_base_context(page, &base_ctx) {
                    Ok(html) => Ok(html),
                    Err(e) => {
                        // Log the error but skip the page instead of aborting the build
                        eprintln!(
                            "  {} (skipping page '{}')",
                            format_template_error(&e),
                            page.slug
                        );
                        Ok(String::new())
                    }
                }
            },
        ),
        profile,
    )?;

    // Print colored build summary
    if !quiet {
        print_build_summary(&report);
    }

    // JSON output for CI
    if json {
        let json_report = serde_json::json!({
            "total_pages": report.total_pages,
            "pages_written": report.pages_written,
            "pages_unchanged": report.pages_unchanged,
            "pages_skipped": report.pages_skipped,
            "elapsed_ms": report.elapsed_ms,
        });
        println!(
            "{}",
            serde_json::to_string(&json_report).unwrap_or_default()
        );
    }

    // --- Post-build ---
    let post_ctx = PostBuildContext {
        site_config,
        output_dir: &output_dir,
        site_data: &site_data,
        assets_value: &assets_value,
        engine: &engine,
    };
    post_build(&post_ctx, &built_pages, &mut plugin_manager, &report, quiet)?;

    if profile && !quiet {
        println!(
            "  {} {}",
            "Full pipeline:".dimmed(),
            format!("{}ms", full_start.elapsed().as_millis()).yellow(),
        );
    }

    Ok(())
}

struct PostBuildContext<'a> {
    site_config: &'a mythic_core::config::SiteConfig,
    output_dir: &'a Path,
    site_data: &'a serde_json::Value,
    assets_value: &'a serde_json::Value,
    engine: &'a mythic_template::TemplateEngine,
}

fn post_build(
    ctx: &PostBuildContext<'_>,
    pages: &[mythic_core::page::Page],
    plugin_manager: &mut mythic_core::plugin::PluginManager,
    report: &mythic_core::build::BuildReport,
    quiet: bool,
) -> Result<()> {
    let PostBuildContext {
        site_config,
        output_dir,
        site_data,
        assets_value,
        engine,
    } = ctx;
    // Detect duplicate slugs
    {
        let mut seen = std::collections::HashMap::new();
        for page in pages {
            if let Some(prev) = seen.insert(&page.slug, &page.source_path) {
                eprintln!(
                    "  {} duplicate slug '{}': {} and {}",
                    "warning:".yellow().bold(),
                    page.slug.yellow(),
                    prev.display(),
                    page.source_path.display(),
                );
            }
        }
    }

    // Handle 404 page: if 404.md was built, copy its output to 404.html at root
    // (most static hosts serve 404.html for missing routes)
    let four_oh_four = output_dir.join("404/index.html");
    if four_oh_four.exists() {
        let html = std::fs::read_to_string(&four_oh_four)?;
        std::fs::write(output_dir.join("404.html"), html)?;
    }

    // Generate redirect pages from aliases
    let redirect_count =
        mythic_core::redirects::generate_redirects(pages, output_dir, &site_config.base_url)?;
    if redirect_count > 0 && !quiet {
        println!("  {} {} redirect(s)", "Generated".dimmed(), redirect_count);
    }

    // Skip heavy post-build work when nothing changed (incremental no-op)
    if report.pages_written == 0 {
        plugin_manager.run_post_build(report)?;
        return Ok(());
    }

    // Generate search index
    mythic_core::search::generate_search_index(pages, output_dir, &site_config.base_url)?;

    // Generate taxonomy pages with pagination
    if !site_config.taxonomies.is_empty() {
        let taxonomies = mythic_core::taxonomy::build_taxonomies(site_config, pages);

        // Render taxonomy listing pages
        let taxonomy_pages = mythic_core::taxonomy::generate_taxonomy_pages(&taxonomies);
        for mut page in taxonomy_pages {
            page.rendered_html = Some(String::new());

            // For term pages, generate paginated versions
            let is_term_page = page.frontmatter.layout.as_deref() == Some("taxonomy_term");
            if is_term_page {
                // Find the matching term to get its pages
                let term_data = taxonomies.iter().find_map(|t| {
                    t.terms.iter().find(|term| {
                        let expected_slug = format!("{}/{}", t.config.slug, term.slug);
                        page.slug == expected_slug
                    })
                });

                if let Some(term) = term_data {
                    // Create lightweight pages for pagination from term's page refs
                    let term_pages: Vec<mythic_core::page::Page> = term
                        .pages
                        .iter()
                        .map(|pr| mythic_core::page::Page {
                            source_path: std::path::PathBuf::new(),
                            slug: pr.slug.clone(),
                            frontmatter: mythic_core::page::Frontmatter {
                                title: pr.title.clone().into(),
                                date: pr.date.as_ref().map(|d| d.clone().into()),
                                ..Default::default()
                            },
                            raw_content: String::new(),
                            rendered_html: None,
                            body_html: None,
                            output_path: None,
                            content_hash: 0,
                            toc: Vec::new(),
                        })
                        .collect();

                    let paginated = mythic_core::pagination::paginate(
                        &term_pages,
                        10,
                        &page.slug,
                        &site_config.base_url,
                    );

                    for (page_num, paginator) in &paginated {
                        let slug = mythic_core::pagination::paginated_slug(&page.slug, *page_num);
                        let mut paged = page.clone();
                        paged.slug = slug.clone();

                        // Merge paginator into site_data for template context
                        let mut extra_ctx = if let serde_json::Value::Object(ref map) = site_data {
                            map.clone()
                        } else {
                            serde_json::Map::new()
                        };
                        let paginator_json = serde_json::to_value(paginator).unwrap_or_default();
                        extra_ctx.insert("paginator".to_string(), paginator_json);
                        let extra_value = serde_json::Value::Object(extra_ctx);

                        if let Ok(html) = engine.render_full(
                            &paged,
                            site_config,
                            Some(assets_value),
                            Some(&extra_value),
                        ) {
                            let dest = output_dir.join(&slug).join("index.html");
                            if let Some(parent) = dest.parent() {
                                std::fs::create_dir_all(parent)?;
                            }
                            std::fs::write(&dest, html)?;
                        }
                    }
                    continue; // Skip the non-paginated render below
                }
            }

            // Non-paginated page (listing pages, or terms without pagination)
            // For listing pages, inject taxonomy terms into the data context
            let render_data = if page.frontmatter.layout.as_deref() == Some("taxonomy_list") {
                let taxonomy_data = taxonomies.iter().find(|t| t.config.slug == page.slug);
                if let Some(taxonomy) = taxonomy_data {
                    let mut extra_ctx = if let serde_json::Value::Object(ref map) = site_data {
                        map.clone()
                    } else {
                        serde_json::Map::new()
                    };
                    let terms_json: Vec<serde_json::Value> = taxonomy
                        .terms
                        .iter()
                        .map(|term| {
                            serde_json::json!({
                                "name": term.name,
                                "slug": term.slug,
                                "url": format!("/{}/{}/", taxonomy.config.slug, term.slug),
                                "count": term.pages.len(),
                            })
                        })
                        .collect();
                    extra_ctx.insert("terms".to_string(), serde_json::Value::Array(terms_json));
                    serde_json::Value::Object(extra_ctx)
                } else {
                    (*site_data).clone()
                }
            } else {
                (*site_data).clone()
            };

            if let Ok(html) =
                engine.render_full(&page, site_config, Some(assets_value), Some(&render_data))
            {
                let dest = output_dir.join(&page.slug).join("index.html");
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, html)?;
            }
        }

        mythic_core::feed::generate_feeds(site_config, pages, &taxonomies, output_dir)?;
    } else if site_config.feed.is_some() {
        mythic_core::feed::generate_feeds(site_config, pages, &[], output_dir)?;
    }

    // Generate sitemap and robots.txt
    mythic_core::sitemap::generate(site_config, pages, output_dir)?;

    plugin_manager.run_post_build(report)?;

    // Compute content diff for minimal deployments
    let diff = mythic_core::diff::compute_diff(output_dir)?;
    if !quiet && diff.total_changes() > 0 {
        print!("{diff}");
        mythic_core::diff::write_deploy_manifest(output_dir, &diff)?;
    }

    Ok(())
}

// --- Dev server ---

async fn cmd_serve(config_path: &Path, port: u16, drafts: bool, open: bool) -> Result<()> {
    let mut site_config = mythic_core::config::load_config(config_path)?;
    let root = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Override base_url to localhost for local development
    site_config.base_url = format!("http://localhost:{port}");

    println!("{}", "Building site...".dimmed());
    if !drafts {
        println!("  {} use --drafts to include draft pages", "tip:".dimmed());
    }
    full_build(&site_config, &root, drafts, false, false, false)?;

    let (reload_tx, _) = mythic_server::server::reload_channel();
    let watcher = mythic_server::watcher::FileWatcher::new(&site_config, &root)?;

    let rebuild_tx = reload_tx.clone();
    let rebuild_config_path = config_path.to_path_buf();
    let rebuild_root = root.clone();
    std::thread::spawn(move || {
        let mut current_config =
            mythic_core::config::load_config(&rebuild_config_path).expect("Failed to load config");
        current_config.base_url = format!("http://localhost:{port}");
        while let Ok(event) = watcher.rx.recv() {
            println!("  {} {event:?}", "Change detected:".cyan());

            // Re-read config on config changes so template context stays current
            if matches!(event, mythic_server::watcher::WatchEvent::ConfigChanged) {
                match mythic_core::config::load_config(&rebuild_config_path) {
                    Ok(mut cfg) => {
                        cfg.base_url = format!("http://localhost:{port}");
                        current_config = cfg;
                    }
                    Err(e) => {
                        eprintln!("  {} {e}", "Config error:".red().bold());
                        continue;
                    }
                }
            }

            match full_build(&current_config, &rebuild_root, drafts, false, true, false) {
                Ok(_) => {
                    use mythic_server::server::{notify_reload, ReloadMessage};
                    use mythic_server::watcher::WatchEvent;

                    let msg = match &event {
                        WatchEvent::CssChanged(p) => ReloadMessage::CssReload {
                            path: p.to_string_lossy().to_string(),
                        },
                        WatchEvent::ContentChanged(_) => ReloadMessage::Reload,
                        _ => ReloadMessage::Reload,
                    };
                    notify_reload(&rebuild_tx, msg);
                }
                Err(e) => {
                    eprintln!("  {} {e}", "Build error:".red().bold());
                    // Send error to browser for display
                    use mythic_server::server::{notify_reload, ReloadMessage};
                    notify_reload(
                        &rebuild_tx,
                        ReloadMessage::Error {
                            message: format!("{e:#}"),
                        },
                    );
                }
            }
        }
    });

    if open {
        let url = format!("http://localhost:{port}");
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }

    mythic_server::server::serve(&site_config, &root, port, reload_tx).await?;

    Ok(())
}

// --- Init / scaffolding ---

// Starters embedded in the binary via include_dir
use include_dir::{include_dir, Dir};
static STARTERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/starters");

fn init_project(name: &str, template: &str) -> Result<()> {
    let root = PathBuf::from(name);

    // First try filesystem starters (development), then embedded starters
    let starters_dir = find_starters_dir();
    let starter_path = starters_dir.as_ref().and_then(|d| {
        let p = d.join(template);
        if p.exists() {
            Some(p)
        } else {
            None
        }
    });

    if let Some(starter) = starter_path {
        // Filesystem starters (development mode)
        copy_dir_recursive(&starter, &root)?;
    } else if let Some(embedded) = STARTERS.get_dir(template) {
        // Embedded starters (installed binary)
        extract_embedded_dir(embedded, &root)?;
    } else {
        // Fallback: minimal blank site
        std::fs::create_dir_all(root.join("content"))?;
        std::fs::create_dir_all(root.join("templates"))?;

        std::fs::write(
            root.join("mythic.toml"),
            format!("title = \"{name}\"\nbase_url = \"http://localhost:3000\"\n"),
        )?;

        std::fs::write(
            root.join("content/index.md"),
            "---\ntitle: Welcome\n---\n# Welcome to your new site\n\nStart editing `content/index.md` to get started.\n",
        )?;

        std::fs::write(
            root.join("templates/default.html"),
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"utf-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n    <title>{{ page.title }} — {{ site.title }}</title>\n</head>\n<body>\n    <main>{{ content | safe }}</main>\n</body>\n</html>\n",
        )?;
    }

    let gitignore = root.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, "/public\n.mythic-cache.json\n")?;
    }

    println!(
        "{} Mythic site in '{}' (template: {})",
        "Created".green().bold(),
        name.bold(),
        template.cyan()
    );
    println!("  cd {name} && mythic serve");

    Ok(())
}

fn find_starters_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let workspace_root = exe.parent()?.parent()?.parent()?;
        let starters = workspace_root.join("starters");
        if starters.exists() {
            return Some(starters);
        }
    }

    let local = PathBuf::from("starters");
    if local.exists() {
        return Some(local);
    }

    None
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap_or(path);
        let target = dest.join(rel);

        if path.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, &target)?;
        }
    }
    Ok(())
}

fn extract_embedded_dir(dir: &include_dir::Dir, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for file in dir.files() {
        let target = dest.join(file.path());
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, file.contents())?;
    }
    for subdir in dir.dirs() {
        extract_embedded_dir(
            subdir,
            &dest.join(subdir.path().file_name().unwrap_or_default()),
        )?;
    }
    Ok(())
}
