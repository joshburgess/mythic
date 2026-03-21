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
    /// Create a new Mythic site
    Init {
        /// Project name
        name: String,
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
            let site_config = mythic_core::config::load_config(&config)?;

            let root = config
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));

            if clean {
                let output = root.join(&site_config.output_dir);
                if output.exists() {
                    std::fs::remove_dir_all(&output)?;
                }
            }

            // Load template engine
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
        }
        Commands::Init { name } => {
            init_project(&name)?;
        }
    }

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
