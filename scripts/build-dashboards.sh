#!/usr/bin/env bash
# Build the admin dashboard assets.
# Output: dashboards/admin/dist/
set -euo pipefail

cd "$(dirname "$0")/../dashboards/admin"

echo "Installing admin dashboard dependencies..."
npm ci

echo "Building admin dashboard..."
npm run build

echo ""
echo "Built: dashboards/admin/dist/"
ls -la dist/
