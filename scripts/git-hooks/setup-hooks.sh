#!/usr/bin/env bash
set -euo pipefail

echo "=== Setting up Git hooks ==="
REPO_ROOT=$(git rev-parse --show-toplevel)

cat <<EOF
ℹ️  This project uses Git submodules but does not enforce hooks via Husky anymore.

Each app submodule manages its own hooks independently.

To set up hooks locally, run:
  cp -r scripts/git-hooks/ .git/hooks/
EOF
