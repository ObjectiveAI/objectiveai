#!/usr/bin/env bash
# Scaffold a complete ObjectiveAI plugin into the CURRENT directory.
#
#   mkdir my-plugin && cd my-plugin
#   curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/scaffold.sh | bash -s -- rust
#
# Produces both halves under ONE manifest:
#
#   my-plugin/
#   ├── objectiveai.json   the manifest — both halves, at the root
#   ├── README.md
#   ├── mcp/               the MCP server half
#   ├── viewer/            the viewer half
#   └── .agents/skills/    every skill, collected here
#
# The NAME is the directory's own name — that is why this takes only a
# language. It becomes the Cargo package, the binary, the MCP server's
# routing prefix, and the npm package name.
#
# Usage:
#   bash scaffold.sh <language>      # rs | rust
#
# Environment:
#   OBJECTIVEAI_SCAFFOLD_REPO   default https://github.com/ObjectiveAI/objectiveai
#   OBJECTIVEAI_SCAFFOLD_REF    default main

set -euo pipefail

REPO="${OBJECTIVEAI_SCAFFOLD_REPO:-https://github.com/ObjectiveAI/objectiveai}"
REF="${OBJECTIVEAI_SCAFFOLD_REF:-main}"

# The name the scaffolds ship with, rewritten to the directory's name.
PLACEHOLDER="objectiveai-plugin-scaffold"

die() { echo "scaffold: $*" >&2; exit 1; }

# ── Language ────────────────────────────────────────────────────────────
# One argument, and it is required rather than defaulted: which language
# the MCP half is written in decides which scaffolds are cloned AND which
# root manifest is used (its build caches are cargo-specific). Naming it
# now keeps the command honest when there is more than one.
[ "$#" -ge 1 ] || die "usage: scaffold.sh <language>   (supported: rs, rust)"
case "$1" in
  rs | rust) ROOT_SCAFFOLD="objectiveai-plugin-scaffold-rs"
             MCP_SCAFFOLD="objectiveai-mcp-plugin-scaffold-rs" ;;
  *) die "unsupported language '$1' (supported: rs, rust)" ;;
esac
VIEWER_SCAFFOLD="objectiveai-viewer-plugin-scaffold"

# ── Name, from the directory ────────────────────────────────────────────
# The same regex the scaffolds' own rename scripts use, and for a real
# reason: this name becomes the MCP server name, from which the proxy
# builds every tool's routing prefix — and that rewrite maps `_` and `.`
# to `-`. A directory name that would be mangled is refused here rather
# than silently normalized into something the author did not choose.
NAME="$(basename "$PWD")"
echo "$NAME" | grep -Eq '^[a-z0-9][a-z0-9-]*$' \
  || die "directory name '$NAME' cannot be a plugin name.
  Allowed: lowercase letters, digits and dashes, starting with a letter
  or digit. Rename the directory and run this again."
[ "$NAME" = "$PLACEHOLDER" ] && die "pick a directory name of your own, not '$PLACEHOLDER'"

# ── Refuse to clobber ───────────────────────────────────────────────────
for existing in objectiveai.json mcp viewer; do
  [ -e "$existing" ] && die "'$existing' already exists here — refusing to overwrite.
  Run this in an empty directory."
done

# ── Fetch ───────────────────────────────────────────────────────────────
# The scaffolds are subdirectories of the monorepo, not standalone repos,
# so this is ONE sparse clone rather than three: blobless, depth 1, and
# checking out only the four paths that matter.
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "scaffold: fetching $REF from $REPO"
git clone --quiet --depth 1 --filter=blob:none --sparse --branch "$REF" \
  "$REPO" "$TMP/src" || die "clone failed"
git -C "$TMP/src" sparse-checkout set --no-cone \
  "/$ROOT_SCAFFOLD/" "/$MCP_SCAFFOLD/" "/$VIEWER_SCAFFOLD/" "/.agents/" \
  >/dev/null || die "sparse-checkout failed"

for required in "$ROOT_SCAFFOLD" "$MCP_SCAFFOLD" "$VIEWER_SCAFFOLD" ".agents"; do
  [ -d "$TMP/src/$required" ] || die "$REF has no '$required' — wrong ref?"
done

# ── Assemble ────────────────────────────────────────────────────────────
# `.` gets the shared root (manifest + README); the halves get their own
# directories, whose names the root manifest's containerfile paths depend
# on. Dotfiles included — the halves ship .gitignore files that are
# already correct relative to their own directory.
echo "scaffold: assembling"
cp -R "$TMP/src/$ROOT_SCAFFOLD/." .
cp -R "$TMP/src/$MCP_SCAFFOLD" mcp
cp -R "$TMP/src/$VIEWER_SCAFFOLD" viewer

# Every skill in ONE place. An agent working here should not have to
# discover three `.agents` directories to learn how the thing it is
# editing gets built, registered and driven.
mkdir -p .agents/skills
cp -R "$TMP/src/.agents/skills/." .agents/skills/
cp -R mcp/.agents/skills/. .agents/skills/
cp -R viewer/.agents/skills/. .agents/skills/
rm -rf mcp/.agents viewer/.agents

# What the root now owns, or what this script replaces.
rm -f mcp/objectiveai.json viewer/objectiveai.json
rm -f mcp/rename.sh viewer/rename.sh

# ── Rename ──────────────────────────────────────────────────────────────
# awk into a temp file, not `sed -i`: GNU and BSD disagree on `-i`, and
# this has to run on both. `Cargo.lock` is in the list because the image
# builds `--locked` — a lockfile still naming the placeholder fails the
# build with a message about nothing you touched.
rename_in() {
  local file="$1" tmp
  [ -f "$file" ] || return 0
  tmp="$(mktemp)"
  awk -v old="$PLACEHOLDER" -v new="$NAME" '{ gsub(old, new); print }' \
    "$file" > "$tmp"
  mv "$tmp" "$file"
}

echo "scaffold: naming it '$NAME'"
for file in \
  objectiveai.json \
  README.md \
  mcp/Cargo.toml \
  mcp/Cargo.lock \
  mcp/Containerfile \
  mcp/src/main.rs \
  mcp/README.md \
  viewer/package.json \
  viewer/src/home.tsx \
  viewer/README.md
do
  rename_in "$file"
done

# ── A plugin is published as a git repo ─────────────────────────────────
if [ ! -e .git ]; then
  git init --quiet
  echo "scaffold: initialized a git repository"
fi

cat <<EOF

Scaffolded '$NAME'.

  objectiveai.json   both halves, one manifest
  mcp/               the MCP server (Rust)
  viewer/            tabs, channel handlers, browser scripts
  .agents/skills/    $(find .agents/skills -name SKILL.md | wc -l | tr -d ' ') skills

Next:

  objectiveai laboratories spawn
  objectiveai development plugins mcp create \\
    --owner you --name $NAME --version v0.1.0 --path "\$PWD"
  objectiveai development plugins viewer create \\
    --owner you --name $NAME --version v0.1.0 --path "\$PWD"

  cd viewer && pnpm install && pnpm run dev

README.md has the rest.
EOF
