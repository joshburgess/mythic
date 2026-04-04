# mythic-template

Multi-engine template system for the [Mythic](https://github.com/joshburgess/mythic) static site generator.

Supports Tera and Handlebars templates side by side in the same project. Includes custom filters for reading time, word count, and Hugo-compatible helpers. Lazy collection functions (`get_pages()`, `get_sections()`) avoid per-page cloning overhead for large sites.

This crate is primarily intended to be used as a dependency of other Mythic crates. For the CLI tool, install [`mythic-cli`](https://crates.io/crates/mythic-cli).

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or [MIT License](../../LICENSE-MIT) at your option.
