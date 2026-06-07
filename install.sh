#!/bin/sh
# ACLI installer — downloads the correct release binary for your platform.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/neuro-ng/acli/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/neuro-ng/acli/main/install.sh | sh -s -- --uninstall
#   ACLI_INSTALL_DIR=~/.local/bin sh install.sh
set -e

REPO="neuro-ng/acli"
BINARY_NAME="acli"
INSTALL_DIR="${ACLI_INSTALL_DIR:-/usr/local/bin}"

die() { echo "Error: $*" >&2; exit 1; }

detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) die "Unsupported OS: $(uname -s)" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) die "Unsupported architecture: $(uname -m)" ;;
    esac
}

detect_target() {
    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"
    case "${os}-${arch}" in
        linux-x86_64)   echo "x86_64-unknown-linux-musl" ;;
        linux-aarch64)  echo "aarch64-unknown-linux-gnu" ;;
        darwin-x86_64)  echo "x86_64-apple-darwin" ;;
        darwin-aarch64) echo "aarch64-apple-darwin" ;;
        windows-x86_64) echo "x86_64-pc-windows-msvc" ;;
        *) die "No prebuilt binary for ${os}-${arch}" ;;
    esac
}

get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSI "https://github.com/${REPO}/releases/latest" \
            | grep -i "^location:" | sed 's#.*/tag/##' | tr -d '\r\n'
    elif command -v wget >/dev/null 2>&1; then
        wget -q --spider --max-redirect=0 \
            "https://github.com/${REPO}/releases/latest" 2>&1 \
            | grep "Location:" | sed 's#.*/tag/##' | tr -d '\r\n'
    else
        die "curl or wget is required"
    fi
}

download() {
    local url="$1" dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$dest" "$url"
    else
        wget -qO "$dest" "$url"
    fi
}

uninstall() {
    os="$(detect_os)"
    if [ "$os" = "windows" ]; then
        dest="${INSTALL_DIR}/${BINARY_NAME}.exe"
    else
        dest="${INSTALL_DIR}/${BINARY_NAME}"
    fi

    [ -f "$dest" ] || die "acli not found at ${dest}"

    if [ -w "$dest" ]; then
        rm "$dest"
    else
        echo "Needs sudo to remove ${dest}"
        sudo rm "$dest"
    fi

    echo "acli uninstalled from ${dest}"
}

install() {
    os="$(detect_os)"
    target="$(detect_target)"
    version="${ACLI_VERSION:-$(get_latest_version)}"

    [ -n "$version" ] || die "Could not determine latest release version"

    if [ "$os" = "windows" ]; then
        dest="${INSTALL_DIR}/${BINARY_NAME}.exe"
        archive="acli-${version}-${target}.zip"
    else
        dest="${INSTALL_DIR}/${BINARY_NAME}"
        archive="acli-${version}-${target}.tar.gz"
    fi

    # Version check — skip download if already up to date.
    if [ -x "$dest" ]; then
        current="$("$dest" version 2>/dev/null | awk '{print $2}')"
        if [ "$current" = "$version" ]; then
            echo "acli ${version} is already installed at ${dest} — nothing to do."
            exit 0
        fi
        echo "Upgrading acli ${current} → ${version}..."
    else
        echo "Installing acli ${version} (${target})..."
    fi

    url="https://github.com/${REPO}/releases/download/${version}/${archive}"
    echo "Downloading ${url}"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    download "$url" "${tmpdir}/${archive}"

    if [ "$os" = "windows" ]; then
        command -v unzip >/dev/null 2>&1 || die "unzip is required"
        unzip -q "${tmpdir}/${archive}" -d "${tmpdir}"
        src="${tmpdir}/${BINARY_NAME}.exe"
    else
        tar -xzf "${tmpdir}/${archive}" -C "${tmpdir}"
        src="${tmpdir}/${BINARY_NAME}"
        chmod +x "$src"
    fi

    if [ -w "$INSTALL_DIR" ]; then
        mv "$src" "$dest"
    else
        echo "Needs sudo to write to ${INSTALL_DIR}"
        sudo mv "$src" "$dest"
    fi

    echo "acli ${version} installed → ${dest}"
    echo "Run 'acli version' to verify."
}

case "${1:-}" in
    --uninstall) uninstall ;;
    *)           install ;;
esac
