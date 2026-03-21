#!/usr/bin/env bash
# Mythic installer — detects OS/arch and downloads the correct binary.
# Usage: curl -fsSL https://raw.githubusercontent.com/joshburgess/mythic/main/install.sh | sh

set -euo pipefail

REPO="joshburgess/mythic"
INSTALL_DIR="${MYTHIC_INSTALL_DIR:-/usr/local/bin}"

get_latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
}

detect_target() {
    local os arch target

    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux)
            case "$arch" in
                x86_64)  target="x86_64-unknown-linux-gnu" ;;
                aarch64) target="aarch64-unknown-linux-gnu" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64)  target="x86_64-apple-darwin" ;;
                arm64)   target="aarch64-apple-darwin" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $os" >&2
            exit 1
            ;;
    esac

    echo "$target"
}

main() {
    local version target url tmpdir

    echo "Installing Mythic..."

    version=$(get_latest_version)
    if [ -z "$version" ]; then
        echo "Error: could not determine latest version" >&2
        exit 1
    fi
    echo "  Version: $version"

    target=$(detect_target)
    echo "  Target:  $target"

    url="https://github.com/$REPO/releases/download/$version/mythic-$target.tar.gz"
    echo "  URL:     $url"

    tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" EXIT

    echo "  Downloading..."
    curl -fsSL "$url" -o "$tmpdir/mythic.tar.gz"

    echo "  Extracting..."
    tar xzf "$tmpdir/mythic.tar.gz" -C "$tmpdir"

    echo "  Installing to $INSTALL_DIR..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "$tmpdir/mythic" "$INSTALL_DIR/mythic"
    else
        sudo mv "$tmpdir/mythic" "$INSTALL_DIR/mythic"
    fi
    chmod +x "$INSTALL_DIR/mythic"

    echo ""
    echo "Mythic $version installed successfully!"
    echo "Run 'mythic --help' to get started."
}

main
