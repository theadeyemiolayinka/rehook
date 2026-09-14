#!/usr/bin/env bash
# Build the HookRelay agent binary for the current platform.
# Output: target/release/hookrelay
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Building hookrelay-agent (release)..."
cargo build --release -p hookrelay-agent

BINARY="target/release/hookrelay"
if [[ "$(uname)" == "Darwin" ]]; then
    BINARY="target/release/hookrelay"
fi

echo ""
echo "Built: $BINARY"
"$BINARY" version
