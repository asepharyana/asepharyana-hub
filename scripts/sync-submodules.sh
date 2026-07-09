#!/usr/bin/env bash
set -euo pipefail

echo "=== Syncing all submodules ==="

git submodule update --init --recursive

echo ""
echo "=== Latest submodule status ==="
git submodule status

echo ""
echo "✅ All submodules synced"
