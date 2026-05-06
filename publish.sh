#!/usr/bin/env bash
# Dispatches every per-package publish.sh in parallel.
#
# Each per-package script triggers its own GitHub Actions workflow (or, for
# objectiveai-go, pushes a git tag locally). All dispatches run concurrently
# — one failure does not abort the others — but this script's exit status
# reflects whether any failed.
#
# Race-condition note: when a fresh version bump is being published,
# downstream crates/packages may race-fail because their upstreams haven't
# landed on the registry yet (e.g. `objectiveai-api` depends on
# `objectiveai`; `objectiveai-cocoindex` depends on `objectiveai-py`). The
# fix is simply to rerun the failed per-package publish.sh once the upstream
# version is live.
#
# Usage:
#   bash publish.sh                # production registries
#   bash publish.sh --test         # test registries where supported (PyPI for py + cocoindex)
#   bash publish.sh --build-only   # local sanity check across all packages

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

PACKAGES=(
  objectiveai-rs-macros
  objectiveai-rs
  objectiveai-api
  objectiveai-mcp-cli
  objectiveai-mcp-proxy
  objectiveai-mcp-filesystem
  objectiveai-py
  objectiveai-cocoindex
  objectiveai-js
  objectiveai-go
  objectiveai-cli
)

PIDS=()
for pkg in "${PACKAGES[@]}"; do
  bash "$REPO_ROOT/$pkg/publish.sh" "$@" &
  PIDS+=($!)
done

FAILED=false
for pid in "${PIDS[@]}"; do
  if ! wait "$pid"; then
    FAILED=true
  fi
done

if $FAILED; then
  exit 1
fi
