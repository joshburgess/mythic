#!/usr/bin/env bash
# Mythic installer — detects OS/arch, downloads the correct binary, and verifies its checksum.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/joshburgess/mythic/main/install.sh | sh
#
# Options (via environment variables):
#   MYTHIC_VERSION      Install a specific version (e.g., "v0.1.0"). Default: latest.
#   MYTHIC_INSTALL_DIR  Install directory. Default: /usr/local/bin.

set -euo pipefail

REPO="joshburgess/mythic"
INSTALL_DIR="${MYTHIC_INSTALL_DIR:-/usr/local/bin}"

get_latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"([^"]+)".*/\1/'
}

detect_target() {
    local os arch

    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64)  echo "x86_64-apple-darwin" ;;
                arm64)   echo "aarch64-apple-darwin" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $os" >&2
            echo "For Windows, download from https://github.com/$REPO/releases" >&2
            exit 1
            ;;
    esac
}

verify_checksum() {
    local file="$1" expected="$2"

    if command -v sha256sum &>/dev/null; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum &>/dev/null; then
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        echo "  Warning: no sha256sum or shasum found, skipping checksum verification" >&2
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        echo "  Error: checksum mismatch!" >&2
        echo "  Expected: $expected" >&2
        echo "  Got:      $actual" >&2
        exit 1
    fi
}

main() {
    echo "Installing Mythic..."

    local version="${MYTHIC_VERSION:-}"
    if [ -z "$version" ]; then
        version=$(get_latest_version)
    fi
    if [ -z "$version" ]; then
        echo "Error: could not determine latest version." >&2
        echo "Set MYTHIC_VERSION=v0.1.0 to install a specific version." >&2
        exit 1
    fi
    echo "  Version: $version"

    local target
    target=$(detect_target)
    echo "  Target:  $target"

    local base_url="https://github.com/$REPO/releases/download/$version"
    local archive="mythic-$target.tar.gz"
    local url="$base_url/$archive"

    local tmpdir
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    echo "  Downloading $archive..."
    if ! curl -fsSL "$url" -o "$tmpdir/$archive"; then
        echo "Error: download failed. Check that version $version exists at:" >&2
        echo "  https://github.com/$REPO/releases/tag/$version" >&2
        exit 1
    fi

    # Verify checksum if available
    echo "  Verifying checksum..."
    if curl -fsSL "$base_url/SHA256SUMS.txt" -o "$tmpdir/SHA256SUMS.txt" 2>/dev/null; then
        expected=$(grep "$archive" "$tmpdir/SHA256SUMS.txt" | awk '{print $1}')
        if [ -n "$expected" ]; then
            verify_checksum "$tmpdir/$archive" "$expected"
            echo "  Checksum verified."
        else
            echo "  Warning: archive not found in SHA256SUMS.txt, skipping verification." >&2
        fi
    else
        echo "  Warning: SHA256SUMS.txt not available, skipping verification." >&2
    fi

    echo "  Extracting..."
    tar xzf "$tmpdir/$archive" -C "$tmpdir"

    if [ ! -f "$tmpdir/mythic" ]; then
        echo "Error: expected 'mythic' binary not found in archive." >&2
        exit 1
    fi

    echo "  Installing to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR" 2>/dev/null || true
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
