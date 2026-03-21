# Contributing to Mythic

## Getting Started

```bash
git clone https://github.com/joshburgess/mythic.git
cd mythic
cargo build --workspace
cargo test --workspace
```

## Project Layout

```
crates/
  mythic-core/       # Core library: config, content, build pipeline, plugins
  mythic-markdown/   # Markdown processing, frontmatter, shortcodes, highlighting
  mythic-template/   # Template engines (Tera + Handlebars)
  mythic-assets/     # Asset pipeline (images, CSS, JS, Sass)
  mythic-server/     # Dev server, file watcher, live reload
  mythic-cli/        # CLI binary
fixtures/            # Test fixture sites
starters/            # Starter templates (blank, blog, docs, portfolio)
docs/                # Documentation site (built with Mythic)
```

## Development Workflow

### Running tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p mythic-core

# Specific test
cargo test -p mythic-core build::tests::incremental
```

### Building the CLI

```bash
# Debug
cargo run -p mythic-cli -- build --config fixtures/basic-site/mythic.toml

# Release
cargo build --release -p mythic-cli
./target/release/mythic-cli --help
```

### Benchmarks

```bash
cargo bench -p mythic-core
```

### Building the docs site

```bash
cargo run -p mythic-cli -- build --config docs/mythic.toml
cargo run -p mythic-cli -- serve --config docs/mythic.toml
```

## Code Conventions

- **Error handling**: Use `anyhow` throughout. Provide context with `.with_context()`.
- **Parallelism**: Use `rayon` for CPU-bound work. Keep filesystem I/O sequential to avoid contention.
- **Config**: All new config fields go in `SiteConfig` with `#[serde(default)]` and a default function. Update `for_testing()`.
- **New features**: Add to the appropriate crate. The CLI orchestrates; crates should be independently useful.
- **Tests**: Every module has inline `#[cfg(test)]` tests. Integration tests go in `tests/`. Aim for both happy path and edge cases.

## Adding a New Feature

1. Add config fields to `mythic-core/src/config.rs` (with defaults and `for_testing()` update)
2. Implement the feature in the appropriate crate
3. Wire it into the build pipeline (`mythic-core/src/build.rs`) or CLI (`mythic-cli/src/main.rs`)
4. Write tests (at minimum: happy path, error case, edge case)
5. Update the docs site if user-facing
6. Update fixture sites if the feature affects the template context

## Pull Requests

- Keep PRs focused on a single feature or fix
- Include tests
- Run `cargo test --workspace` before submitting
- Run `cargo clippy --workspace` and fix warnings
- Run `cargo fmt --all` for formatting
