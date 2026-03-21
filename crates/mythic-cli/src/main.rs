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
        } => {
            cmd_build(&config, drafts, clean)?;
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
        Commands::Init { name } => {
            init_project(&name)?;
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

fn cmd_build(config_path: &PathBuf, drafts: bool, clean: bool) -> Result<()> {
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

    mythic_core::build::build(
        &site_config,
        root,
        drafts,
        |pages| mythic_markdown::render::render_markdown(pages),
        Some(|page: &mythic_core::page::Page, cfg: &mythic_core::config::SiteConfig| {
            engine.render(page, cfg)
        }),
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

fn init_project(name: &str) -> Result<()> {
    let root = PathBuf::from(name);
    std::fs::create_dir_all(root.join("content"))?;
    std::fs::create_dir_all(root.join("templates"))?;

    std::fs::write(
        root.join("mythic.toml"),
        format!(
            r#"title = "{name}"
base_url = "http://localhost:3000"
"#
        ),
    )?;

    std::fs::write(
        root.join("content/index.md"),
        r#"---
title: Welcome
---
# Welcome to your new site

Start editing `content/index.md` to get started.
"#,
    )?;

    std::fs::write(
        root.join("templates/default.html"),
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{{ page.title }} — {{ site.title }}</title>
</head>
<body>
    <main>
        {{ content | safe }}
    </main>
</body>
</html>
"#,
    )?;

    std::fs::write(
        root.join(".gitignore"),
        "/public\n.mythic-cache.json\n",
    )?;

    println!("Created new Mythic site in '{name}'");
    println!("  cd {name} && mythic build");

    Ok(())
}
