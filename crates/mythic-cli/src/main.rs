//! CLI binary for the Mythic static site generator.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "mythic", about = "A fast static site generator written in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            config,
            drafts,
            clean,
            profile,
        } => {
            let site_config = mythic_core::config::load_config(&config)?;
            let root = config
                .parent()
                .unwrap_or_else(|| Path::new("."));

            if clean {
                let output = root.join(&site_config.output_dir);
                if output.exists() {
                    std::fs::remove_dir_all(&output)?;
                }
            }

            full_build(&site_config, root, drafts, profile)?;
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
        Commands::Check { config } => {
            let site_config = mythic_core::config::load_config(&config)?;
            let root = config
                .parent()
                .unwrap_or_else(|| Path::new("."));
            let output_dir = root.join(&site_config.output_dir);

            let report = mythic_core::check::check_site(&output_dir)?;
            report.print_summary();

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
                "eleventy" | "11ty" => mythic_core::migrate::eleventy::migrate(&source, &output)?,
                other => anyhow::bail!("Unknown source SSG: {other}. Supported: jekyll, hugo, eleventy"),
            };
            report.print_summary();
        }
    }

    Ok(())
}

/// Run the full build pipeline with all features integrated.
fn full_build(
    site_config: &mythic_core::config::SiteConfig,
    root: &Path,
    drafts: bool,
    profile: bool,
) -> Result<()> {
    let output_dir = root.join(&site_config.output_dir);

    // --- Pre-build: load data, plugins ---

    // Load data files
    let data_dir = root.join(&site_config.data_dir);
    let site_data = mythic_core::data::load_data(&data_dir)?;

    // Load plugins (Rust built-in + Rhai scripts)
    let mut plugin_manager = mythic_core::plugin::PluginManager::new();
    plugin_manager.register(Box::new(mythic_core::plugin::ReadingTimePlugin::new()));

    let plugins_dir = root.join("plugins");
    if plugins_dir.exists() {
        let rhai_plugins = mythic_core::rhai_plugin::load_rhai_plugins(&plugins_dir)?;
        for plugin in rhai_plugins {
            plugin_manager.register(plugin);
        }
    }

    // Run pre-build hooks
    plugin_manager.run_pre_build(site_config)?;

    // Load template engine
    let template_dir = root.join(&site_config.template_dir);
    let default_engine = site_config
        .templates
        .as_ref()
        .map(|t| t.default_engine.as_str())
        .unwrap_or("tera");
    let engine = mythic_template::TemplateEngine::new_with_default(&template_dir, default_engine)?;

    // Prepare render config
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
        toc_min_level: site_config
            .toc
            .as_ref()
            .map(|t| t.min_level)
            .unwrap_or(2),
        toc_max_level: site_config
            .toc
            .as_ref()
            .map(|t| t.max_level)
            .unwrap_or(4),
    };

    // Prepare shortcode dir
    let shortcode_dir = root.join("shortcodes");

    // Build combined context for templates (assets + data)
    let assets_manifest = mythic_assets::process_assets(site_config, root)?;
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

    // Generate syntax highlight CSS
    let highlighter = mythic_markdown::highlight::Highlighter::new(
        &render_config.highlight_theme,
        render_config.line_numbers,
    );
    let highlight_css = highlighter.generate_css();
    if !highlight_css.is_empty() {
        let css_dir = output_dir.clone();
        std::fs::create_dir_all(&css_dir)?;
        std::fs::write(css_dir.join("syntax.css"), &highlight_css)?;
    }

    // --- Core build with integrated pipeline ---

    let report = mythic_core::build::build_with_profile(
        site_config,
        root,
        drafts,
        |pages| {
            // Apply data cascade
            let content_dir = root.join(&site_config.content_dir);
            if let Err(e) = mythic_core::cascade::apply_cascade(pages, &content_dir) {
                eprintln!("  Cascade error: {e}");
            }

            // Run on_page_discovered hooks
            if let Err(e) = plugin_manager.run_all_discovered(pages) {
                eprintln!("  Plugin error: {e}");
            }

            // Process shortcodes before markdown rendering
            for page in pages.iter_mut() {
                if let Err(e) = plugin_manager.run_pre_render(page) {
                    eprintln!("  Plugin pre_render error: {e}");
                }

                if shortcode_dir.exists() {
                    match mythic_markdown::shortcodes::process_shortcodes(
                        &page.raw_content,
                        &shortcode_dir,
                    ) {
                        Ok(processed) => page.raw_content = processed,
                        Err(e) => eprintln!("  Shortcode error in {}: {e}", page.slug),
                    }
                }
            }

            // Process i18n
            if let Some(ref i18n_config) = site_config.i18n {
                mythic_core::i18n::process_i18n(pages, i18n_config);
            }

            // Render markdown with syntax highlighting and TOC
            mythic_markdown::render::render_markdown_with_config(pages, &render_config);

            // Run post_render hooks
            for page in pages.iter_mut() {
                if let Err(e) = plugin_manager.run_post_render(page) {
                    eprintln!("  Plugin post_render error: {e}");
                }
            }
        },
        Some(|page: &mythic_core::page::Page, cfg: &mythic_core::config::SiteConfig| {
            engine.render_full(page, cfg, Some(&assets_value), Some(&site_data))
        }),
        profile,
    )?;

    // --- Post-build: taxonomies, feeds, sitemap ---

    // Re-discover content for taxonomy/feed generation (need the rendered pages)
    // We re-read because the build function consumed them. For taxonomies we
    // only need the frontmatter data, not the rendered HTML.
    let all_pages = mythic_core::content::discover_content(site_config, root)?;
    let non_draft_pages: Vec<_> = all_pages
        .into_iter()
        .filter(|p| !p.frontmatter.draft.unwrap_or(false) || drafts)
        .collect();

    // Generate taxonomy pages
    if !site_config.taxonomies.is_empty() {
        let taxonomies = mythic_core::taxonomy::build_taxonomies(site_config, &non_draft_pages);
        let taxonomy_pages = mythic_core::taxonomy::generate_taxonomy_pages(&taxonomies);

        // Render and write taxonomy pages
        for mut page in taxonomy_pages {
            page.rendered_html = Some(String::new()); // Empty content for listing pages
            match engine.render(&page, site_config) {
                Ok(html) => {
                    let dest = output_dir.join(&page.slug).join("index.html");
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&dest, html)?;
                }
                Err(_) => {
                    // Template may not exist (taxonomy_list.html / taxonomy_term.html)
                    // This is expected if the user hasn't created taxonomy templates
                }
            }
        }

        // Generate feeds for taxonomies
        mythic_core::feed::generate_feeds(
            site_config,
            &non_draft_pages,
            &taxonomies,
            &output_dir,
        )?;
    } else if site_config.feed.is_some() {
        // Site-wide feed only (no taxonomies)
        mythic_core::feed::generate_feeds(site_config, &non_draft_pages, &[], &output_dir)?;
    }

    // Generate sitemap and robots.txt
    mythic_core::sitemap::generate(site_config, &non_draft_pages, &output_dir)?;

    // Run post-build hooks
    plugin_manager.run_post_build(&report)?;

    Ok(())
}

async fn cmd_serve(config_path: &PathBuf, port: u16, drafts: bool, open: bool) -> Result<()> {
    let site_config = mythic_core::config::load_config(config_path)?;
    let root = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Initial build with full pipeline
    println!("Building site...");
    full_build(&site_config, &root, drafts, false)?;

    // Set up reload channel
    let (reload_tx, _) = mythic_server::server::reload_channel();

    // Start file watcher
    let watcher = mythic_server::watcher::FileWatcher::new(&site_config, &root)?;

    // Spawn rebuild loop
    let rebuild_tx = reload_tx.clone();
    let rebuild_config = site_config.clone();
    let rebuild_root = root.clone();
    std::thread::spawn(move || {
        while let Ok(event) = watcher.rx.recv() {
            println!("  Change detected: {event:?}");

            match full_build(&rebuild_config, &rebuild_root, drafts, false) {
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
                    eprintln!("  Build error: {e}");
                }
            }
        }
    });

    if open {
        let url = format!("http://localhost:{port}");
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }

    // Start server (blocks until Ctrl+C)
    mythic_server::server::serve(&site_config, &root, port, reload_tx).await?;

    Ok(())
}

fn init_project(name: &str, template: &str) -> Result<()> {
    let root = PathBuf::from(name);

    let starters_dir = find_starters_dir();
    let starter_path = starters_dir.as_ref().and_then(|d| {
        let p = d.join(template);
        if p.exists() { Some(p) } else { None }
    });

    if let Some(starter) = starter_path {
        copy_dir_recursive(&starter, &root)?;
    } else {
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

    println!("Created new Mythic site in '{name}' (template: {template})");
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
