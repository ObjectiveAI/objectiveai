#!/usr/bin/env bash
# clear-cache.sh — wipe regenerable build caches.
#
# Every path that gets removed is enumerated below. The list is exhaustive
# at the time of writing; if you add a new cache, also add it here.
#
# Preserves (intentional — these are consumed by builds, NOT caches):
#
#   - node_modules/      (pnpm/npm/yarn output — you should NOT need to
#                        re-run `pnpm install` after this script)
#   - venv/  .venv/      (Python virtualenvs — you should NOT need to
#                        rebuild the env / reinstall deps. We DO sweep
#                        __pycache__ inside them so they stay tidy.)
#   - .logs/             (build/test/publish log capture — keep history)
#   - .next/             (Next.js incremental build cache — keep)
#   - **/embed/          (embedded runner binaries built by *-runner/build.sh
#                        and read by objectiveai-api's build.rs)
#   - **/dist/           (SDK build outputs: objectiveai-sdk-rs-cffi/dist,
#                        objectiveai-sdk-rs-wasm-js/dist,
#                        objectiveai-sdk-js/dist)
#   - **/pkg-nodejs/     (wasm-pack nodejs target output, dist-like)
#   - bin/               (workspace-local cargo-installed tools managed by
#                        build-bin.sh — wasm-pack, maturin)
#   - .crates.toml,
#     .crates2.json      (paired with bin/; cargo-install registry — drop
#                        these and cargo thinks the tools are missing)
#   - Source, lockfiles (Cargo.lock, pnpm-lock.yaml, go.sum, requirements.txt),
#                        config (.env*, .git/), committed generated code.
#
# Usage:
#   bash clear-cache.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# Snapshot disk before so we can report freed space.
freed_before=$(df -k "$REPO_ROOT" | awk 'NR==2 {print $4}')

# ---------------------------------------------------------------------------
# Helper: empty a directory's contents, but leave the directory itself in
# place. This preserves any junctions/symlinks pointing at an external drive
# (the target/ tree usually lives on D: via an NTFS junction) AND the dir
# entry on disk, so re-running a build doesn't have to recreate the link.
#
# Uses a shell glob (`shopt -s dotglob nullglob; rm -rf -- "$dir"/*`) rather
# than `find`. Bash's glob expansion traverses NTFS junctions transparently —
# `find -mindepth 1 -delete` silently no-ops on junction children under
# msys2/git-bash on Windows and would leave the contents intact.
# ---------------------------------------------------------------------------
empty_dir() {
  local dir="$1"
  # `-e` follows the symlink — true iff the target exists. The link-itself
  # check (`-L`) is intentionally separate so a dangling link is recreated
  # as a fresh empty dir below, not left as a broken link.
  if [ -L "$dir" ] && [ ! -e "$dir" ]; then
    echo "Dangling symlink $dir/ — replacing with empty dir"
    rm -f -- "$dir"
    mkdir -p -- "$dir"
    return
  fi
  if [ ! -e "$dir" ]; then
    return
  fi
  echo "Emptying $dir/"
  # Shell glob in a subshell so the `shopt` changes don't leak out:
  #   dotglob   — match dotfiles (.cargo-lock, .fingerprint, ...)
  #   nullglob  — empty dirs expand to nothing instead of a literal `*`
  (
    shopt -s dotglob nullglob
    rm -rf -- "$dir"/*
  )
}

# ---------------------------------------------------------------------------
# Cargo build outputs. The workspace target/ is usually an NTFS junction to
# D:\Programming\objectiveai-targets\target so the 100+ GB of build artifacts
# don't fill the system drive. The other target-* dirs follow the same
# convention. We empty contents only — the junction (or plain dir) stays
# in place so the next build writes straight to the same destination.
# ---------------------------------------------------------------------------
RUST_TARGETS=(
  target
  target-objectiveai-mcp-filesystem
  target-objectiveai-mcp-proxy
  target-objectiveai-viewer
)

for t in "${RUST_TARGETS[@]}"; do
  empty_dir "$t"
done

# ---------------------------------------------------------------------------
# Top-level singleton caches. Same "empty contents, keep the dir" policy as
# the Rust targets above so any tool watching these paths doesn't see them
# disappear.
# ---------------------------------------------------------------------------
TOP_LEVEL_CACHES=(
  .pytest_cache
  objectiveai-cocoindex/.pytest_cache
  objectiveai-sdk-py/.pytest_cache
  objectiveai-claude-agent-sdk-runner/.pyinstaller-work
  objectiveai-codex-sdk-runner/.pyinstaller-work
)

for d in "${TOP_LEVEL_CACHES[@]}"; do
  empty_dir "$d"
done

# ---------------------------------------------------------------------------
# Python bytecode (__pycache__) — swept inside each Python source tree we
# own AND inside each venv (the venv stays, but its __pycache__ goes).
# Each root is listed explicitly so this stays auditable; the find within
# a root is bounded by that root.
# ---------------------------------------------------------------------------
PY_SOURCE_ROOTS=(
  objectiveai-claude-agent-sdk-runner
  objectiveai-claude-agent-sdk-runner/venv
  objectiveai-codex-sdk-runner
  objectiveai-codex-sdk-runner/venv
  objectiveai-cocoindex/objectiveai_cocoindex
  objectiveai-cocoindex/tests
  objectiveai-cocoindex/venv
  objectiveai-github-discord-notifier
  objectiveai-github-discord-notifier/.venv
  objectiveai-sdk-go/venv
  objectiveai-sdk-py/objectiveai_sdk
  objectiveai-sdk-py/scripts
  objectiveai-sdk-py/tests
  objectiveai-sdk-py/venv
)

for root in "${PY_SOURCE_ROOTS[@]}"; do
  [ -d "$root" ] || continue
  # Print the root once, then remove every __pycache__ within it.
  count=$(find "$root" -type d -name __pycache__ 2>/dev/null | wc -l)
  if [ "$count" -gt 0 ]; then
    echo "Removing $count __pycache__ under $root/"
    find "$root" -type d -name __pycache__ -prune -exec rm -rf -- {} +
  fi
done

freed_after=$(df -k "$REPO_ROOT" | awk 'NR==2 {print $4}')
delta_gb=$(awk -v b="$freed_before" -v a="$freed_after" \
  'BEGIN { printf "%.1f", (a - b) / 1024 / 1024 }')

echo
echo "Done. Freed ${delta_gb} GiB on the repo's filesystem."
