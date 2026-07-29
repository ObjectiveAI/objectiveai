#!/usr/bin/env bash
# Rename the scaffold: rewrites `objectiveai-plugin-scaffold` in the
# files that carry it. Usage: ./rename.sh <new-name>
set -euo pipefail

NEW="${1:?usage: ./rename.sh <new-name>}"
if ! [[ "$NEW" =~ ^[a-z0-9][a-z0-9-]*$ ]]; then
  echo "error: name must match ^[a-z0-9][a-z0-9-]*$" >&2
  exit 1
fi

OLD="objectiveai-plugin-scaffold"
for f in package.json README.md src/home.tsx \
  .agents/skills/plugin-development/SKILL.md; do
  sed -i "s/${OLD}/${NEW}/g" "$f"
done
echo "renamed ${OLD} -> ${NEW}"
