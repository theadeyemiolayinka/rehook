#!/usr/bin/env bash
# HookRelay agent installer.
#
# Downloads the latest release binary for your platform from GitHub,
# installs it to /usr/local/bin (or a local bin dir), and verifies it runs.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/scripts/install-agent.sh | bash
#
# Or to install a specific version:
#   curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/scripts/install-agent.sh | bash -s -- --version v0.1.0
#
# To update an existing installation:
#   hookrelay version
#   curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/scripts/install-agent.sh | bash
#
# The script is safe to re-run. It overwrites the existing binary.

set -euo pipefail

REPO="theadeyemiolayinka/hookrelay"
GITHUB_API="https://api.github.com/repos/${REPO}"
INSTALL_DIR="/usr/local/bin"
LOCAL_DIR="${HOME}/.local/bin"
VERSION=""

# Parse arguments.
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: install-agent.sh [--version <tag>] [--dir <path>]"
            echo ""
            echo "Installs the HookRelay agent binary from GitHub releases."
            echo ""
            echo "Options:"
            echo "  --version <tag>   Install a specific release tag (e.g. v0.1.0)"
            echo "  --dir <path>      Install directory (default: /usr/local/bin)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Detect platform.
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)   PLATFORM_OS="linux" ;;
    Darwin)  PLATFORM_OS="darwin" ;;
    *)
        echo "Unsupported OS: $OS"
        echo "Supported: linux, darwin (macOS)"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)   PLATFORM_ARCH="x86_64" ;;
    aarch64|arm64)  PLATFORM_ARCH="aarch64" ;;
    *)
        echo "Unsupported architecture: $ARCH"
        echo "Supported: x86_64, aarch64"
        exit 1
        ;;
esac

# Determine the version to install.
if [[ -z "$VERSION" ]]; then
    echo "Fetching latest release..."
    VERSION=$(curl -fsSL "${GITHUB_API}/releases/latest" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)
    if [[ -z "$VERSION" ]]; then
        echo "Could not determine latest release. Check your network or specify --version."
        exit 1
    fi
fi

echo "Installing HookRelay agent ${VERSION} for ${PLATFORM_OS}-${PLATFORM_ARCH}..."

# Construct the download URL. The release asset naming is:
#   hookrelay-{version}-{os}-{arch}
# The binary inside the tarball is named "hookrelay".
ASSET_NAME="hookrelay-${VERSION}-${PLATFORM_OS}-${PLATFORM_ARCH}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"

# Download to a temp directory.
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading ${DOWNLOAD_URL}..."
if ! curl -fsSL -o "${TMP_DIR}/${ASSET_NAME}" "$DOWNLOAD_URL"; then
    echo ""
    echo "Download failed. The asset may not exist for this platform/version."
    echo "Check available assets at: https://github.com/${REPO}/releases/tag/${VERSION}"
    exit 1
fi

echo "Extracting..."
tar -xzf "${TMP_DIR}/${ASSET_NAME}" -C "$TMP_DIR"

# Find the binary.
BINARY_PATH="${TMP_DIR}/hookrelay"
if [[ ! -f "$BINARY_PATH" ]]; then
    # Some release layouts put it in a subdirectory.
    BINARY_PATH=$(find "$TMP_DIR" -name "hookrelay" -type f | head -1)
    if [[ -z "$BINARY_PATH" ]]; then
        echo "Could not find the hookrelay binary in the archive."
        exit 1
    fi
fi

chmod +x "$BINARY_PATH"

# Verify it runs.
VERSION_OUTPUT=$("$BINARY_PATH" version 2>&1) || true
echo "  Binary version: ${VERSION_OUTPUT}"

# Choose install directory.
if [[ -w "$INSTALL_DIR" ]] || [[ $EUID -eq 0 ]]; then
    TARGET="${INSTALL_DIR}/hookrelay"
else
    echo ""
    echo "${INSTALL_DIR} is not writable. Installing to ${LOCAL_DIR} instead."
    mkdir -p "$LOCAL_DIR"
    TARGET="${LOCAL_DIR}/hookrelay"

    # Warn if not in PATH.
    case ":${PATH}:" in
        *":${LOCAL_DIR}:"*) ;;
        *)
            echo ""
            echo "WARNING: ${LOCAL_DIR} is not in your PATH."
            echo "Add this to your shell profile:"
            echo "  export PATH=\"${LOCAL_DIR}:\$PATH\""
            ;;
    esac
fi

# Install.
mv "$BINARY_PATH" "$TARGET"
echo ""
echo "Installed: ${TARGET}"
"$TARGET" version

echo ""
echo "Next steps:"
echo "  hookrelay web    # open the agent web UI in your browser"
echo "  hookrelay --help # see all commands"
echo ""
echo "Docs: https://theadeyemiolayinka.github.io/hookrelay/"
