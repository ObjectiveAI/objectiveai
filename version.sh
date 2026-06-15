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
#   test-upstream           = { path = "test-upstream", version = "X.Y.Z" }
# ...and rewrites the `version = "..."` token inside. The `test-upstream`
# alternative covers the workspace-internal sibling crate that
# objectiveai-mcp-proxy depends on for its dev tests.
set_cargo_objectiveai_deps() {
  local file="$1"
  inline_substitute "$file" \
    '^(objectiveai(-[a-zA-Z0-9_-]+)?|test-upstream)[[:space:]]*=' \
    'version = "[0-9][^"]*"' \
    "version = \"$NEW_VERSION\""
}

# Bare-string `objectiveai-sdk = "X.Y.Z"` deps. Used in README install snippets
# that demonstrate Cargo.toml entries to downstream users. Distinct from
# `set_cargo_objectiveai_deps` which only handles the inline-table form
# `objectiveai-sdk = { ..., version = "X.Y.Z", ... }`.
set_objectiveai_string_dep() {
  local file="$1"
  inline_substitute "$file" \
    '^objectiveai-sdk[[:space:]]*=[[:space:]]*"[0-9]' \
    '"[0-9][0-9.]*"' \
    "\"$NEW_VERSION\""
}

# `version: 'X.Y.Z'` property lines in TypeScript / JavaScript. Used by
# files whose runtime identifier (e.g. an MCP client `name`+`version`
# pair) should track the package version. Matches every `version: '...'`
# in the file — list the file here only if all such occurrences share
# the package version.
set_ts_version_string() {
  local file="$1"
  inline_substitute "$file" \
    "version:[[:space:]]*'[0-9]" \
    "'[0-9][0-9.]*'" \
    "'$NEW_VERSION'"
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

# `objectiveai-sdk==X.Y.Z` lines in a pip requirements.txt. Other entries in
# the file (including non-pinned ones, comments, and unrelated packages) pass
# through untouched. Spec stays `==NEW_VERSION`.
set_requirements_objectiveai_pin() {
  local file="$1"
  inline_substitute "$file" \
    '^objectiveai-sdk[[:space:]]*==' \
    '==[0-9][^[:space:]]*' \
    "==$NEW_VERSION"
}

# `"objectiveai-sdk==X.Y.Z"` entries inside a pyproject.toml [project.dependencies]
# array (or any line that quotes an objectiveai-sdk pin). The token regex stops
# at the closing quote/comma so trailing TOML punctuation isn't consumed. Lines
# without an objectiveai-sdk pin pass through untouched.
set_pyproject_objectiveai_dep_pin() {
  local file="$1"
  inline_substitute "$file" \
    'objectiveai-sdk[[:space:]]*==' \
    '==[0-9][^",[:space:]]*' \
    "==$NEW_VERSION"
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

# Overwrite a single-line version file (the Go SDK's independent
# version source — see objectiveai-sdk-go/publish.sh). The lockstep
# bump moves it along with everything else by default; edit the file
# alone to release (or hold back) the Go SDK independently.
set_version_txt() {
  local file="$1"
  printf '%s\n' "$NEW_VERSION" > "$file"
}

# ---------------------------------------------------------------------------
# File lists
# ---------------------------------------------------------------------------

CARGO_TOMLS=(
  objectiveai-api/Cargo.toml
  objectiveai-cli/Cargo.toml
  objectiveai-cli/test-fixtures/hello-plugin/Cargo.toml
  objectiveai-db/Cargo.toml
  objectiveai-json-schema/builder/Cargo.toml
  objectiveai-mcp/Cargo.toml
  objectiveai-mcp-filesystem/Cargo.toml
  objectiveai-mcp-proxy/Cargo.toml
  objectiveai-mcp-proxy/test-upstream/Cargo.toml
  objectiveai-sdk-rs/Cargo.toml
  objectiveai-sdk-rs-cffi/Cargo.toml
  objectiveai-sdk-rs-macros/Cargo.toml
  objectiveai-sdk-rs-pyo3/Cargo.toml
  objectiveai-sdk-rs-wasm-js/Cargo.toml
  objectiveai-viewer/src-tauri/Cargo.toml
)

PYPROJECT_TOMLS=(
  objectiveai-sdk-py/pyproject.toml
  objectiveai-cocoindex/pyproject.toml
)

PACKAGE_JSONS=(
  objectiveai-sdk-js/package.json
  objectiveai-function-tree/package.json
  objectiveai-viewer/package.json
  objectiveai-mcp-proxy/tests-ts/package.json
)

CSPROJS=(
  objectiveai-dotnet/ObjectiveAI/ObjectiveAI.csproj
)

PY_RUNNER_MAINS=(
  objectiveai-claude-agent-sdk-runner/main.py
  objectiveai-codex-sdk-runner/main.py
)

# pip requirements.txt files that pin `objectiveai==X.Y.Z`.
REQUIREMENTS_TXTS=(
  objectiveai-cocoindex/requirements.txt
)

# Markdown files that embed bare-string `objectiveai = "X.Y.Z"` Cargo
# dep snippets (typically install instructions for the Rust SDK).
MARKDOWN_FILES=(
  README.md
)

# TypeScript/JavaScript files that embed a literal `version: 'X.Y.Z'`
# property tied to the workspace version (e.g. MCP client identifiers
# in test rigs). All `version: '...'` lines in each listed file get
# bumped — verify there are no unrelated occurrences before adding.
TS_VERSION_STRING_FILES=(
  objectiveai-mcp-proxy/tests-ts/src/rig.ts
)

# Single-line version files for packages that version independently of
# the manifests above (currently: the Go SDK, whose releases are git
# tags derived from this file).
VERSION_TXTS=(
  objectiveai-sdk-go/version.txt
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
      set_pyproject_objectiveai_dep_pin "$file"
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
    reqs)
      set_requirements_objectiveai_pin "$file"
      ;;
    md)
      set_objectiveai_string_dep "$file"
      ;;
    ts)
      set_ts_version_string "$file"
      ;;
    vertxt)
      set_version_txt "$file"
      ;;
  esac
}

echo "Setting version to $NEW_VERSION"

for rel in "${CARGO_TOMLS[@]}";           do update cargo  "$rel"; done
for rel in "${PYPROJECT_TOMLS[@]}";        do update pypro  "$rel"; done
for rel in "${PACKAGE_JSONS[@]}";          do update pkg    "$rel"; done
for rel in "${CSPROJS[@]}";                do update csproj "$rel"; done
for rel in "${PY_RUNNER_MAINS[@]}";        do update pyrun  "$rel"; done
for rel in "${REQUIREMENTS_TXTS[@]}";       do update reqs   "$rel"; done
for rel in "${MARKDOWN_FILES[@]}";          do update md     "$rel"; done
for rel in "${TS_VERSION_STRING_FILES[@]}"; do update ts     "$rel"; done
for rel in "${VERSION_TXTS[@]}";            do update vertxt "$rel"; done

# Sync Cargo.lock to the new workspace versions. If we leave Cargo.lock
# with the old versions, every cargo invocation in CI rewrites the
# lockfile mid-build, which mutates files mid-run and breaks fingerprint
# checks (objectiveai-api/build.rs runs validate.sh that hashes Cargo.lock).
if command -v cargo >/dev/null 2>&1; then
  echo
  echo "Refreshing Cargo.lock workspace versions..."
  ( cd "$REPO_ROOT" && cargo update -w >/dev/null 2>&1 ) && echo "  Cargo.lock synced." \
    || echo "  Cargo.lock refresh failed — run 'cargo update -w' manually."
else
  echo
  echo "WARNING: cargo not found on PATH. Run 'cargo update -w' manually before"
  echo "         pushing — otherwise CI will mutate Cargo.lock mid-build and"
  echo "         break fingerprint checks."
fi

echo
echo "Done. pnpm-lock.yaml will refresh on next pnpm install."
