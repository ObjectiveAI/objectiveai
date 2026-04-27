#!/usr/bin/env bash
# version.sh — set the version of every ObjectiveAI package (and inter-package
# dependency reference) to a single value. Touches Rust crates, Python
# packages, JS packages, .NET csproj, and the runner subpackages. Skips
# lockfiles (Cargo.lock, pnpm-lock.yaml) — those regenerate on next build.
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

# Cargo.toml / pyproject.toml [package|project] version.
# Convention in our repo: the first `^version = "..."` line is the package
# version. Third-party dependency version specs never appear at column 0
# without a leading `= {` so this targets the right line.
set_toml_package_version() {
  local file="$1"
  first_line_replace "$file" \
    '^version = "[^"]+"' \
    "version = \"$NEW_VERSION\""
}

# Inter-package Cargo.toml dependency version pins.
# Matches lines like:
#   objectiveai             = { path = "..", version = "X.Y.Z", ... }
#   objectiveai-api         = { version = "X.Y.Z", ... }
#   objectiveai-cli         = { ... }
# ...and rewrites the `version = "..."` token inside.
set_cargo_objectiveai_deps() {
  local file="$1"
  inline_substitute "$file" \
    '^objectiveai(-[a-zA-Z0-9_-]+)?[[:space:]]*=' \
    'version = "[0-9][^"]*"' \
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

# Ensure a package.json has a "version" field; insert one right after "name"
# if missing.
ensure_package_json_version() {
  local file="$1"
  if grep -q '^[[:space:]]*"version":' "$file"; then
    set_package_json_version "$file"
  else
    insert_after_first "$file" \
      '^[[:space:]]*"name":' \
      "  \"version\": \"$NEW_VERSION\","
  fi
}

# .csproj <Version>...</Version>
set_csproj_version() {
  local file="$1"
  inline_substitute "$file" \
    '<Version>' \
    '<Version>[^<]*</Version>' \
    "<Version>$NEW_VERSION</Version>"
}

# __version__ = "..." in a Python file. If absent, insert one right after
# `from __future__ import annotations` (every runner has that line) with a
# blank line before it for PEP 8 readability.
ensure_py_module_version() {
  local file="$1"
  if grep -q '^__version__ = ' "$file"; then
    first_line_replace "$file" \
      '^__version__ = ' \
      "__version__ = \"$NEW_VERSION\""
  else
    local tmp
    tmp=$(mktemp)
    awk -v ver="$NEW_VERSION" '
      { print }
      !done && /^from __future__ import annotations/ {
        print ""
        printf "__version__ = \"%s\"\n", ver
        done=1
      }
    ' "$file" > "$tmp"
    mv "$tmp" "$file"
  fi
}

# ---------------------------------------------------------------------------
# File lists
# ---------------------------------------------------------------------------

CARGO_TOMLS=(
  objectiveai-api/Cargo.toml
  objectiveai-cli/Cargo.toml
  objectiveai-cli/builder/Cargo.toml
  objectiveai-json-schema/builder/Cargo.toml
  objectiveai-mcp/Cargo.toml
  objectiveai-rs/Cargo.toml
  objectiveai-rs-cffi/Cargo.toml
  objectiveai-rs-macros/Cargo.toml
  objectiveai-rs-pyo3/Cargo.toml
  objectiveai-rs-wasm-js/Cargo.toml
  objectiveai-viewer/src-tauri/Cargo.toml
)

PYPROJECT_TOMLS=(
  objectiveai-rs-pyo3/pyproject.toml
)

PACKAGE_JSONS=(
  objectiveai-js/package.json
  objectiveai-function-tree/package.json
  objectiveai-viewer/package.json
)

PACKAGE_JSONS_OPTIONAL=(
  objectiveai-codex-sdk-runner-js/package.json
)

CSPROJS=(
  objectiveai-dotnet/ObjectiveAI/ObjectiveAI.csproj
)

PY_RUNNER_MAINS=(
  objectiveai-claude-agent-sdk-runner-py/main.py
  objectiveai-codex-sdk-runner-py/main.py
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
      set_cargo_objectiveai_deps "$file"
      ;;
    pypro)
      set_toml_package_version "$file"
      ;;
    pkg)
      set_package_json_version "$file"
      ;;
    pkg+)
      ensure_package_json_version "$file"
      ;;
    csproj)
      set_csproj_version "$file"
      ;;
    pyrun)
      ensure_py_module_version "$file"
      ;;
  esac
}

echo "Setting version to $NEW_VERSION"

for rel in "${CARGO_TOMLS[@]}";           do update cargo  "$rel"; done
for rel in "${PYPROJECT_TOMLS[@]}";        do update pypro  "$rel"; done
for rel in "${PACKAGE_JSONS[@]}";          do update pkg    "$rel"; done
for rel in "${PACKAGE_JSONS_OPTIONAL[@]}"; do update pkg+   "$rel"; done
for rel in "${CSPROJS[@]}";                do update csproj "$rel"; done
for rel in "${PY_RUNNER_MAINS[@]}";        do update pyrun  "$rel"; done

echo
echo "Done. Cargo.lock and pnpm-lock.yaml will refresh on next build."
