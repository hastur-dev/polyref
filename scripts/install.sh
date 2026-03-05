#!/usr/bin/env bash
set -euo pipefail

# Polyref installer for macOS and Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/hastur-dev/polyref/main/scripts/install.sh | bash

REPO="hastur-dev/polyref"
INSTALL_DIR="${POLYREF_INSTALL_DIR:-$HOME/.local/bin}"

detect_platform() {
    local os arch suffix

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)
            echo "Error: Unsupported OS: $os" >&2
            echo "This installer supports Linux and macOS. For Windows, use install.ps1" >&2
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)
            echo "Error: Unsupported architecture: $arch" >&2
            exit 1
            ;;
    esac

    suffix="${os}-${arch}"
    echo "$suffix"
}

get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed 's/.*"tag_name": *"//;s/".*//'
}

main() {
    echo "Polyref Installer"
    echo "================="
    echo

    local platform
    platform="$(detect_platform)"
    echo "Platform: $platform"

    local version="${1:-}"
    if [ -z "$version" ]; then
        echo "Fetching latest version..."
        version="$(get_latest_version)"
        if [ -z "$version" ]; then
            echo "Error: Could not determine latest version." >&2
            echo "You can install from source instead: cargo install --path ." >&2
            exit 1
        fi
    fi
    echo "Version:  $version"

    local url="https://github.com/${REPO}/releases/download/${version}/polyref-${platform}.tar.gz"
    echo "URL:      $url"
    echo

    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    echo "Downloading..."
    if ! curl -fsSL "$url" -o "$tmpdir/polyref.tar.gz"; then
        echo "Error: Download failed." >&2
        echo "Check that version '$version' exists at:" >&2
        echo "  https://github.com/${REPO}/releases" >&2
        echo >&2
        echo "Alternatively, install from source:" >&2
        echo "  cargo install --path ." >&2
        exit 1
    fi

    echo "Extracting..."
    tar xzf "$tmpdir/polyref.tar.gz" -C "$tmpdir"

    echo "Installing to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"

    for bin in polyref polyref-gen polyref-drift; do
        if [ -f "$tmpdir/$bin" ]; then
            cp "$tmpdir/$bin" "$INSTALL_DIR/$bin"
            chmod +x "$INSTALL_DIR/$bin"
            echo "  Installed $bin"
        fi
    done

    echo
    echo "Done!"

    # Check if install dir is in PATH
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo
        echo "WARNING: $INSTALL_DIR is not in your PATH."
        echo "Add it by running:"
        echo
        local shell_name
        shell_name="$(basename "${SHELL:-bash}")"
        case "$shell_name" in
            zsh)  echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc && source ~/.zshrc" ;;
            fish) echo "  fish_add_path $INSTALL_DIR" ;;
            *)    echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc && source ~/.bashrc" ;;
        esac
    fi

    echo
    echo "Verify installation:"
    echo "  polyref --version"
    echo "  polyref-gen --version"
}

main "$@"
