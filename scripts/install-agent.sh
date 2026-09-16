#!/usr/bin/env bash
# Rehook agent installer.
#
# Downloads the release binary for your platform from GitHub, verifies the
# SHA-256 checksum (and the minisign signature when minisign is installed),
# and installs it to a standard bin directory.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash
#
# Or to install a specific version:
#   curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash -s -- --version v1.0.0
#
# The script is safe to re-run. It overwrites the existing binary, which is
# how updates are applied.

set -euo pipefail

REPO="theadeyemiolayinka/rehook"
GITHUB_API="https://api.github.com/repos/${REPO}"
SYSTEM_BIN="/usr/local/bin"
LOCAL_BIN="${HOME}/.local/bin"
VERSION=""

# Public key for verifying release signatures. Populated with the project
# minisign public key; when empty, signature verification is skipped.
MINISIGN_PUB="RWQMZ+gJzEXrYMNO4+1MqhRkFteil2i0o21dTLFDdiYEkUesov9uoARL"

say()  { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

# Parse arguments.
INSTALL_DIR=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="$2"; shift 2 ;;
        --dir)
            INSTALL_DIR="$2"; shift 2 ;;
        --help|-h)
            say "Usage: install-agent.sh [--version <tag>] [--dir <path>]"
            say ""
            say "Installs the Rehook agent binary from GitHub releases."
            say ""
            say "Options:"
            say "  --version <tag>   Install a specific release tag (e.g. v1.0.0)"
            say "  --dir <path>      Install directory (default: /usr/local/bin or ~/.local/bin)"
            exit 0
            ;;
        *) die "unknown option: $1" ;;
    esac
done

# Detect platform.
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)  PLATFORM_OS="linux" ;;
    Darwin) PLATFORM_OS="darwin" ;;
    *)      die "unsupported OS: $OS (supported: linux, darwin)" ;;
esac

case "$ARCH" in
    x86_64|amd64)  PLATFORM_ARCH="x86_64" ;;
    aarch64|arm64) PLATFORM_ARCH="aarch64" ;;
    *)             die "unsupported architecture: $ARCH (supported: x86_64, aarch64)" ;;
esac

# Resolve the version to install.
if [[ -z "$VERSION" ]]; then
    say "Fetching latest release..."
    VERSION=$(curl -fsSL "${GITHUB_API}/releases/latest" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)
    [[ -n "$VERSION" ]] || die "could not determine latest release; specify --version"
fi

say "Installing Rehook agent ${VERSION} for ${PLATFORM_OS}-${PLATFORM_ARCH}..."

# Release asset naming contract (produced by the release workflow):
#   rehook-{tag}-{os}-{arch}.tar.gz
#   rehook-{tag}-{os}-{arch}.tar.gz.sha256
#   rehook-{tag}-{os}-{arch}.tar.gz.minisig
ASSET="rehook-${VERSION}-${PLATFORM_OS}-${PLATFORM_ARCH}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

say "Downloading ${ASSET}..."
curl -fsSL -o "${TMP_DIR}/${ASSET}" "${BASE_URL}/${ASSET}" \
    || die "download failed; check assets at https://github.com/${REPO}/releases/tag/${VERSION}"

# Verify the SHA-256 checksum. This is mandatory: the checksum file is
# published alongside every release asset.
say "Verifying checksum..."
curl -fsSL -o "${TMP_DIR}/${ASSET}.sha256" "${BASE_URL}/${ASSET}.sha256" \
    || die "checksum file missing for ${ASSET}; refusing to install"
EXPECTED=$(cut -d' ' -f1 < "${TMP_DIR}/${ASSET}.sha256")
if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL=$(sha256sum "${TMP_DIR}/${ASSET}" | cut -d' ' -f1)
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL=$(shasum -a 256 "${TMP_DIR}/${ASSET}" | cut -d' ' -f1)
else
    die "no sha256sum or shasum available; cannot verify the download"
fi
[[ "$EXPECTED" == "$ACTUAL" ]] || die "checksum mismatch for ${ASSET}; refusing to install"

# Verify the minisign signature when a public key is configured and
# minisign is installed. Install minisign to enable signature checks.
if [[ -n "$MINISIGN_PUB" ]]; then
    if command -v minisign >/dev/null 2>&1; then
        curl -fsSL -o "${TMP_DIR}/${ASSET}.minisig" "${BASE_URL}/${ASSET}.minisig" \
            || die "signature file missing for ${ASSET}; refusing to install"
        minisign -Vm "${TMP_DIR}/${ASSET}" -P "${MINISIGN_PUB}" -x "${TMP_DIR}/${ASSET}.minisig" \
            || die "signature verification failed for ${ASSET}"
        say "Signature verified."
    else
        warn "minisign not installed; skipping signature verification (checksum already verified)"
    fi
fi

say "Extracting..."
tar -xzf "${TMP_DIR}/${ASSET}" -C "$TMP_DIR"

BINARY_PATH="${TMP_DIR}/rehook"
[[ -f "$BINARY_PATH" ]] || BINARY_PATH=$(find "$TMP_DIR" -name "rehook" -type f | head -1)
[[ -n "$BINARY_PATH" && -f "$BINARY_PATH" ]] || die "could not find the rehook binary in the archive"
chmod +x "$BINARY_PATH"

# Choose the install directory: an explicit --dir, then /usr/local/bin when
# writable, otherwise ~/.local/bin.
if [[ -n "$INSTALL_DIR" ]]; then
    mkdir -p "$INSTALL_DIR"
elif [[ -w "$SYSTEM_BIN" ]] || [[ $EUID -eq 0 ]]; then
    INSTALL_DIR="$SYSTEM_BIN"
else
    INSTALL_DIR="$LOCAL_BIN"
    mkdir -p "$INSTALL_DIR"
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) ;;
        *) warn "${INSTALL_DIR} is not in your PATH; add: export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
    esac
fi

TARGET="${INSTALL_DIR}/rehook"
mv "$BINARY_PATH" "$TARGET"

# Create the agent data and config directories with owner-only permissions.
# The local db and stored credentials live here.
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/rehook"
CONFIG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/rehook"
mkdir -p "$DATA_DIR" "$CONFIG_DIR"
chmod 700 "$DATA_DIR" "$CONFIG_DIR" 2>/dev/null || true

say ""
say "Installed: ${TARGET}"
"$TARGET" version || true

say ""
say "Next steps:"
say "  rehook web      # open the agent web UI in your browser"
say "  rehook --help   # see all commands"
say ""
say "Docs: https://rehook.bytao.dev/"
