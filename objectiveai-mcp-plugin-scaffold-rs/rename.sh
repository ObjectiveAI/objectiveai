#!/usr/bin/env bash
# Turn the scaffold into your plugin.
#
# Rewrites the base name in the four places it appears: the package
# name, the binary name, the `cp` in the Containerfile that copies that
# binary out, and the `NAME` constant the agent's tool prefix is
# derived from.
#
# Usage:
#   ./rename.sh my-plugin

set -euo pipefail

OLD="objectiveai-mcp-plugin-scaffold"
NEW="${1:-}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -z "$NEW" ]; then
  echo "Usage: $0 <plugin-name>" >&2
  echo "Example: $0 my-plugin" >&2
  exit 1
fi

# The name becomes a cargo package name AND the routing prefix
# ObjectiveAI prepends to every tool an agent sees, so keep it to what
# both accept.
if ! printf '%s' "$NEW" | grep -Eq '^[a-z0-9][a-z0-9-]*$'; then
  echo "'$NEW' must be lowercase letters, digits and dashes, starting with a letter or digit." >&2
  exit 1
fi

if [ "$NEW" = "$OLD" ]; then
  echo "That is already the name." >&2
  exit 1
fi

if ! grep -q "$OLD" "$HERE/Cargo.toml"; then
  echo "Already renamed — '$OLD' is not in Cargo.toml." >&2
  exit 1
fi

# awk rather than sed -i: the two disagree between GNU and BSD, and a
# scaffold that only renames itself on Linux is half a scaffold.
rewrite() {
  local file="$1" tmp
  tmp=$(mktemp)
  awk -v old="$OLD" -v new="$NEW" '
    { gsub(old, new); print }
  ' "$file" > "$tmp"
  mv "$tmp" "$file"
  echo "  $(basename "$file")"
}

echo "Renaming $OLD -> $NEW"
rewrite "$HERE/Cargo.toml"
rewrite "$HERE/Containerfile"
rewrite "$HERE/src/main.rs"

echo
echo "Done. Now:"
echo "  cargo run          # starts on the port in objectiveai.json"
echo "  rm rename.sh       # it has done its job"
