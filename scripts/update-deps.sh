#!/usr/bin/env bash
set -euo pipefail

echo "=== Updating root dependencies ==="
bun update

echo ""
echo "=== Updating app submodule dependencies ==="
for app in apps/*/; do
  if [ -f "${app}package.json" ]; then
    echo "→ $app"
    (cd "$app" && bun update 2>/dev/null && echo "  ✓ $app updated") || echo "  - skipping $app (no bun setup)"
  fi
done

echo ""
echo "✅ All dependencies updated"
