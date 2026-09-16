#!/usr/bin/env bash
# Build the Rehook agent binary for the current platform.
# Output: target/release/rehook
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building rehook-agent (release)..."
cargo build --release -p rehook-agent

BINARY="target/release/rehook"
if [[ "$(uname)" == "Darwin" ]]; then
    BINARY="target/release/rehook"
fi

echo ""
echo "Built: $BINARY"
"$BINARY" version
