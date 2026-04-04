# mythic-cli

Command-line interface for the [Mythic](https://github.com/joshburgess/mythic) static site generator.

## Install

```bash
cargo install mythic-cli
```

Or download a prebuilt binary:

```bash
curl -fsSL https://raw.githubusercontent.com/joshburgess/mythic/main/install.sh | sh
```

## Usage

```bash
mythic init my-site --template blog    # Create a new site
cd my-site
mythic serve                           # Dev server with live reload
mythic build                           # Build for production
```

See the [full documentation](https://github.com/joshburgess/mythic) for all commands and features.

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or [MIT License](../../LICENSE-MIT) at your option.
