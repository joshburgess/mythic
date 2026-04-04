# mythic-server

Development server for the [Mythic](https://github.com/joshburgess/mythic) static site generator.

Provides a local dev server built on axum with WebSocket-based live reload, CSS injection without full page refresh, file watching via notify, and error overlay support. Automatically detects changes to content, templates, config, and styles.

This crate is primarily intended to be used as a dependency of other Mythic crates. For the CLI tool, install [`mythic-cli`](https://crates.io/crates/mythic-cli).

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or [MIT License](../../LICENSE-MIT) at your option.
