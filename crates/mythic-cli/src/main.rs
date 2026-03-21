//! CLI binary for the Mythic static site generator.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
            cmd_build(&config, drafts, clean, profile)?;
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
                .unwrap_or_else(|| std::path::Path::new("."));
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

fn cmd_build(config_path: &PathBuf, drafts: bool, clean: bool, profile: bool) -> Result<()> {
    let site_config = mythic_core::config::load_config(config_path)?;
    let root = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    if clean {
        let output = root.join(&site_config.output_dir);
        if output.exists() {
            std::fs::remove_dir_all(&output)?;
        }
    }

    let template_dir = root.join(&site_config.template_dir);
    let engine = mythic_template::TemplateEngine::new(&template_dir)?;

    mythic_core::build::build_with_profile(
        &site_config,
        root,
        drafts,
        |pages| mythic_markdown::render::render_markdown(pages),
        Some(|page: &mythic_core::page::Page, cfg: &mythic_core::config::SiteConfig| {
            engine.render(page, cfg)
        }),
        profile,
    )?;

    Ok(())
}

async fn cmd_serve(config_path: &PathBuf, port: u16, drafts: bool, open: bool) -> Result<()> {
    let site_config = mythic_core::config::load_config(config_path)?;
    let root = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();

    // Initial build
    println!("Building site...");
    let template_dir = root.join(&site_config.template_dir);
    let engine = mythic_template::TemplateEngine::new(&template_dir)?;

    mythic_core::build::build(
        &site_config,
        &root,
        drafts,
        |pages| mythic_markdown::render::render_markdown(pages),
        Some(|page: &mythic_core::page::Page, cfg: &mythic_core::config::SiteConfig| {
            engine.render(page, cfg)
        }),
    )?;

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

            let template_dir = rebuild_root.join(&rebuild_config.template_dir);
            let engine = match mythic_template::TemplateEngine::new(&template_dir) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("  Template error: {e}");
                    continue;
                }
            };

            match mythic_core::build::build(
                &rebuild_config,
                &rebuild_root,
                drafts,
                |pages| mythic_markdown::render::render_markdown(pages),
                Some(
                    |page: &mythic_core::page::Page,
                     cfg: &mythic_core::config::SiteConfig| {
                        engine.render(page, cfg)
                    },
                ),
            ) {
                Ok(_) => {
                    use mythic_server::server::{notify_reload, ReloadMessage};
                    use mythic_server::watcher::WatchEvent;

                    let msg = match &event {
                        WatchEvent::CssChanged(p) => ReloadMessage::CssReload {
                            path: p.to_string_lossy().to_string(),
                        },
                        WatchEvent::ContentChanged(_) => {
                            // TODO: for content-only changes, could send HtmlUpdate
                            // with the new <main> content. For now, full reload.
                            ReloadMessage::Reload
                        }
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

    // Check for bundled starter templates
    let starters_dir = find_starters_dir();
    let starter_path = starters_dir.as_ref().and_then(|d| {
        let p = d.join(template);
        if p.exists() { Some(p) } else { None }
    });

    if let Some(starter) = starter_path {
        copy_dir_recursive(&starter, &root)?;
    } else {
        // Fallback: generate a minimal blank site
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

    // Always add .gitignore
    let gitignore = root.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, "/public\n.mythic-cache.json\n")?;
    }

    println!("Created new Mythic site in '{name}' (template: {template})");
    println!("  cd {name} && mythic serve");

    Ok(())
}

fn find_starters_dir() -> Option<PathBuf> {
    // Check relative to the binary, then common install paths
    if let Ok(exe) = std::env::current_exe() {
        // Development: binary is in target/debug or target/release
        let workspace_root = exe
            .parent()? // debug/release
            .parent()? // target
            .parent()?; // workspace root
        let starters = workspace_root.join("starters");
        if starters.exists() {
            return Some(starters);
        }
    }

    // Check current directory
    let local = PathBuf::from("starters");
    if local.exists() {
        return Some(local);
    }

    None
}

fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) -> Result<()> {
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
