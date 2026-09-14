#!/usr/bin/env bash
# Build the HookRelay server binary for the current platform.
# Output: target/release/hookrelay-server
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building hookrelay-server (release)..."
cargo build --release -p hookrelay-server

BINARY="target/release/hookrelay-server"

echo ""
echo "Built: $BINARY"
"$BINARY" --version
