#!/usr/bin/env bash
# Rehook server installer.
#
# Downloads the rehook-server binary for your platform, verifies the
# SHA-256 checksum (and the minisign signature when minisign is installed),
# installs it to a standard bin directory, and creates a data directory.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-server.sh | bash
#
# Options:
#   --version <tag>   Install a specific release tag (e.g. v1.0.0)
#   --dir <path>      Install directory (default: /usr/local/bin or ~/.local/bin)
#   --data-dir <path> Data directory (default: /var/lib/rehook as root,
#                     otherwise ~/.local/share/rehook-server)
#   --systemd         Write a systemd unit and enable the service (root only)
#
# The script is safe to re-run. It overwrites the existing binary.

set -euo pipefail

REPO="theadeyemiolayinka/rehook"
GITHUB_API="https://api.github.com/repos/${REPO}"
SYSTEM_BIN="/usr/local/bin"
LOCAL_BIN="${HOME}/.local/bin"
VERSION=""
INSTALL_DIR=""
DATA_DIR=""
WITH_SYSTEMD=0

# Public key for verifying release signatures. Populated with the project
# minisign public key; when empty, signature verification is skipped.
MINISIGN_PUB="RWQMZ+gJzEXrYMNO4+1MqhRkFteil2i0o21dTLFDdiYEkUesov9uoARL"

say()  { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)  VERSION="$2"; shift 2 ;;
        --dir)      INSTALL_DIR="$2"; shift 2 ;;
        --data-dir) DATA_DIR="$2"; shift 2 ;;
        --systemd)  WITH_SYSTEMD=1; shift ;;
        --help|-h)
            sed -n '2,20p' "$0"
            exit 0
            ;;
        *) die "unknown option: $1" ;;
    esac
done

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
    *)             die "unsupported architecture: $ARCH" ;;
esac

if [[ -z "$VERSION" ]]; then
    say "Fetching latest release..."
    VERSION=$(curl -fsSL "${GITHUB_API}/releases/latest" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)
    [[ -n "$VERSION" ]] || die "could not determine latest release; specify --version"
fi

say "Installing Rehook server ${VERSION} for ${PLATFORM_OS}-${PLATFORM_ARCH}..."

ASSET="rehook-server-${VERSION}-${PLATFORM_OS}-${PLATFORM_ARCH}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

say "Downloading ${ASSET}..."
curl -fsSL -o "${TMP_DIR}/${ASSET}" "${BASE_URL}/${ASSET}" \
    || die "download failed; check assets at https://github.com/${REPO}/releases/tag/${VERSION}"

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

BINARY_PATH="${TMP_DIR}/rehook-server"
[[ -f "$BINARY_PATH" ]] || BINARY_PATH=$(find "$TMP_DIR" -name "rehook-server" -type f | head -1)
[[ -n "$BINARY_PATH" && -f "$BINARY_PATH" ]] || die "could not find the rehook-server binary in the archive"
chmod +x "$BINARY_PATH"

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

TARGET="${INSTALL_DIR}/rehook-server"
mv "$BINARY_PATH" "$TARGET"

# Data directory: /var/lib/rehook when running as root, otherwise a
# per-user directory. SQLite lives here.
if [[ -z "$DATA_DIR" ]]; then
    if [[ $EUID -eq 0 ]]; then
        DATA_DIR="/var/lib/rehook"
    else
        DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}/rehook-server"
    fi
fi
mkdir -p "$DATA_DIR"
chmod 700 "$DATA_DIR" 2>/dev/null || true

say ""
say "Installed: ${TARGET}"
say "Data dir:  ${DATA_DIR}"

# Optional systemd unit for running the server as a service.
UNIT_PATH="/etc/systemd/system/rehook-server.service"
if [[ "$WITH_SYSTEMD" -eq 1 ]]; then
    if [[ $EUID -ne 0 ]]; then
        die "--systemd requires root"
    fi
    if ! command -v systemctl >/dev/null 2>&1; then
        die "--systemd requires systemd"
    fi

    # Generate a session key if one is not already configured.
    KEY_FILE="/etc/rehook/env"
    mkdir -p /etc/rehook
    chmod 700 /etc/rehook
    if [[ ! -f "$KEY_FILE" ]]; then
        SESSION_KEY=$(openssl rand -hex 32 2>/dev/null || head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')
        cat > "$KEY_FILE" <<EOF
REHOOK_SESSION_KEY=${SESSION_KEY}
REHOOK_DATA_DIR=${DATA_DIR}
REHOOK_LISTEN_ADDR=0.0.0.0:8080
REHOOK_PUBLIC_BASE_URL=http://localhost:8080
EOF
        chmod 600 "$KEY_FILE"
        say "Wrote ${KEY_FILE} (edit REHOOK_PUBLIC_BASE_URL to your domain)"
    fi

    cat > "$UNIT_PATH" <<EOF
[Unit]
Description=Rehook server
After=network.target

[Service]
Type=simple
EnvironmentFile=${KEY_FILE}
ExecStart=${TARGET}
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${DATA_DIR}
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF
    systemctl daemon-reload
    say ""
    say "Systemd unit written to ${UNIT_PATH}"
    say "  Edit ${KEY_FILE}: set REHOOK_PUBLIC_BASE_URL, ADMIN_USERNAME, ADMIN_PASSWORD"
    say "  systemctl enable --now rehook-server"
else
    say ""
    say "Run it directly:"
    say "  rehook-server --listen-addr 0.0.0.0:8080 --data-dir ${DATA_DIR} \\"
    say "    --public-base-url https://hooks.example.com"
    say ""
    say "Or with environment variables:"
    say "  export REHOOK_SESSION_KEY=\$(openssl rand -hex 32)"
    say "  export ADMIN_USERNAME=admin ADMIN_PASSWORD=..."
    say "  rehook-server"
    say ""
    say "Pass --systemd (as root) to install a systemd unit instead."
fi

say ""
say "Docs: https://theadeyemiolayinka.github.io/rehook/"
