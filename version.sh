#!/usr/bin/env bash
# version.sh — set the version of every Diverge package to a single value.
# Touches Rust crates and JS packages. Skips pnpm-lock.yaml, which
# regenerates on the next pnpm install.
#
# Usage:
#   bash version.sh <new-version>
# Example:
#   bash version.sh 2.1.0

set -euo pipefail

if [ "$#" -ne 1 ] || [ -z "${1:-}" ]; then
  echo "Usage: $0 <new-version>" >&2
  echo "Example: $0 2.1.0" >&2
  exit 1
fi

NEW_VERSION="$1"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------------------------------------------------------------------------
# Primitives
# ---------------------------------------------------------------------------
# Everything is awk-based so the script is portable between GNU and BSD
# toolchains (macOS sed's `-i` flag and GNU sed's `0,/pat/` range syntax
# don't agree).

# Rewrite the first line matching $pat with the literal replacement $repl.
# If no match is found, the file is left unchanged (no error).
first_line_replace() {
  local file="$1"
  local pat="$2"
  local repl="$3"
  local tmp
  tmp=$(mktemp)
  awk -v pat="$pat" -v repl="$repl" '
    !done && $0 ~ pat { print repl; done=1; next }
    { print }
  ' "$file" > "$tmp"
  mv "$tmp" "$file"
}

# On each line matching $line_pat, replace every occurrence of $token_pat
# with $token_repl. Other lines pass through unchanged.
inline_substitute() {
  local file="$1"
  local line_pat="$2"
  local token_pat="$3"
  local token_repl="$4"
  local tmp
  tmp=$(mktemp)
  awk -v lp="$line_pat" -v tp="$token_pat" -v tr="$token_repl" '
    $0 ~ lp { gsub(tp, tr) }
    { print }
  ' "$file" > "$tmp"
  mv "$tmp" "$file"
}

# Insert $insert_line right after the first line matching $pat. Idempotent
# guard left to the caller — this always inserts if the anchor matches.
insert_after_first() {
  local file="$1"
  local pat="$2"
  local insert_line="$3"
  local tmp
  tmp=$(mktemp)
  awk -v pat="$pat" -v ins="$insert_line" '
    { print }
    !done && $0 ~ pat { print ins; done=1 }
  ' "$file" > "$tmp"
  mv "$tmp" "$file"
}

# ---------------------------------------------------------------------------
# Per-file-type updaters
# ---------------------------------------------------------------------------

# Cargo.toml [package] version.
# Convention in our repo: the first `^version = "..."` line is the package
# version. Third-party dependency version specs never appear at column 0
# without a leading `= {` so this targets the right line.
set_toml_package_version() {
  local file="$1"
  first_line_replace "$file" \
    '^version = "[^"]+"' \
    "version = \"$NEW_VERSION\""
}

# Root "version": "..." in a package.json. Relies on the standard layout
# where "version" is the first occurrence in the file (right after "name").
set_package_json_version() {
  local file="$1"
  first_line_replace "$file" \
    '^[[:space:]]*"version":[[:space:]]*"[^"]+"' \
    "  \"version\": \"$NEW_VERSION\","
}

# ---------------------------------------------------------------------------
# File lists
# ---------------------------------------------------------------------------

CARGO_TOMLS=(
  diverge-agents/cc/Cargo.toml
  diverge-agents/hermes/Cargo.toml
  diverge-container-proxy/Cargo.toml
  diverge-agents/openrouter/Cargo.toml
  diverge-sdk-rs/Cargo.toml
  diverge-provider/Cargo.toml
  diverge-postgres/Cargo.toml
  diverge-daemon/Cargo.toml
  diverge-cli/Cargo.toml
)

PACKAGE_JSONS=(
  diverge-provider-web/package.json
  diverge-sdk-js/package.json
)

# ---------------------------------------------------------------------------
# Apply
# ---------------------------------------------------------------------------

update() {
  local kind="$1"
  local rel="$2"
  local file="$REPO_ROOT/$rel"
  if [ ! -f "$file" ]; then
    echo "  skip   $rel (not found)"
    return
  fi
  echo "  $kind  $rel"
  case "$kind" in
    cargo)
      set_toml_package_version "$file"
      ;;
    pkg)
      set_package_json_version "$file"
      ;;
  esac
}

echo "Setting version to $NEW_VERSION"

for rel in "${CARGO_TOMLS[@]}";   do update cargo "$rel"; done
for rel in "${PACKAGE_JSONS[@]}"; do update pkg   "$rel"; done

# Sync Cargo.lock to the new workspace versions, so that no later cargo
# invocation rewrites the lockfile mid-build.
if command -v cargo >/dev/null 2>&1; then
  echo
  echo "Refreshing Cargo.lock workspace versions..."
  ( cd "$REPO_ROOT" && cargo update -w >/dev/null 2>&1 ) && echo "  Cargo.lock synced." \
    || echo "  Cargo.lock refresh failed — run 'cargo update -w' manually."
else
  echo
  echo "WARNING: cargo not found on PATH. Run 'cargo update -w' manually before"
  echo "         pushing."
fi

echo
echo "Done. pnpm-lock.yaml will refresh on next pnpm install."
