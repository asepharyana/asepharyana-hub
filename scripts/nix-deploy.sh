#!/usr/bin/env bash
# ── Nix Deploy — VPS-side script ──
# Runs after `nix copy --to ssh://VPS ./result` from CI
# Usage: sudo ./deploy.sh <service-name>
set -euo pipefail

SERVICE="$1"
PROFILE="/nix/var/nix/profiles/${SERVICE}"

# Find the latest store path for this service
LATEST=$(ls -1d /nix/store/*-"${SERVICE}"-* 2>/dev/null | tail -1)
if [ -z "$LATEST" ]; then
  echo "ERROR: No store path found for ${SERVICE}"
  exit 1
fi

# Update profile
/nix/var/nix/profiles/default/bin/nix-env --profile "$PROFILE" --set "$LATEST"

# Restart service
systemctl daemon-reload
systemctl enable --now "${SERVICE}" 2>/dev/null || systemctl restart "${SERVICE}"

echo "Deployed ${SERVICE}: ${LATEST}"
systemctl is-active "${SERVICE}"

# Collect garbage (safe: only removes unreachable paths)
# nix-collect-garbage -d 2>/dev/null || true
