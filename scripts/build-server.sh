#!/usr/bin/env bash
# Build the Rehook server binary for the current platform.
# Output: target/release/rehook-server
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building rehook-server (release)..."
cargo build --release -p rehook-server

BINARY="target/release/rehook-server"

echo ""
echo "Built: $BINARY"
"$BINARY" --version
