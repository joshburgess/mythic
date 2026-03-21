---
title: "Installation"
---

# Installation

Mythic runs on macOS, Linux, and Windows. Choose the installation method that works best for you.

## Install via Cargo

If you have Rust installed, the simplest approach is to install Mythic through Cargo:

```bash
cargo install mythic
```

This compiles Mythic from source and places the binary in your Cargo bin directory (usually `~/.cargo/bin/`). Make sure this directory is in your `PATH`.

To install a specific version:

```bash
cargo install mythic@0.8.0
```

To update to the latest version:

```bash
cargo install mythic --force
```

### Build from Source

You can also clone the repository and build directly:

```bash
git clone https://github.com/mythic-ssg/mythic.git
cd mythic
cargo build --release
```

The compiled binary will be at `target/release/mythic`. Move it somewhere on your `PATH`:

```bash
cp target/release/mythic /usr/local/bin/
```

## Install Script

A one-liner install script is available for macOS and Linux. It detects your platform and downloads the correct binary:

```bash
curl -fsSL https://mythic.site/install.sh | sh
```

To install to a custom directory:

```bash
curl -fsSL https://mythic.site/install.sh | sh -s -- --prefix=/opt/mythic
```

The script installs to `/usr/local/bin` by default.

## Binary Downloads

Pre-built binaries are available on the [GitHub releases page](https://github.com/mythic-ssg/mythic/releases). Download the archive for your platform:

| Platform            | Archive                              |
|---------------------|--------------------------------------|
| macOS (Apple Silicon) | `mythic-aarch64-apple-darwin.tar.gz` |
| macOS (Intel)       | `mythic-x86_64-apple-darwin.tar.gz`  |
| Linux (x86_64)      | `mythic-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64)       | `mythic-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64)    | `mythic-x86_64-pc-windows-msvc.zip` |

Extract and move the binary to a directory on your `PATH`:

```bash
# macOS / Linux
tar -xzf mythic-x86_64-unknown-linux-gnu.tar.gz
chmod +x mythic
sudo mv mythic /usr/local/bin/

# Windows (PowerShell)
Expand-Archive mythic-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item mythic.exe C:\Users\you\.local\bin\
```

## Homebrew (macOS)

```bash
brew install mythic-ssg/tap/mythic
```

## Nix

A Nix flake is available:

```bash
nix run github:mythic-ssg/mythic
```

Or add to your `flake.nix` inputs:

```nix
{
  inputs.mythic.url = "github:mythic-ssg/mythic";
}
```

## Verifying Your Installation

After installing, verify that Mythic is available:

```bash
mythic --version
```

You should see output like:

```
mythic 0.8.0
```

## System Requirements

- **Rust 1.75+** (if building from source)
- **Operating System:** macOS 12+, Linux (glibc 2.31+), or Windows 10+
- **Disk:** ~30 MB for the binary
- **Memory:** Varies by site size; a typical 1,000-page site uses ~100 MB during builds

## Next Steps

With Mythic installed, head to the [Quickstart](/getting-started/quickstart/) to create your first site.
