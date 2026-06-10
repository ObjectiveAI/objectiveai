#!/usr/bin/env bash
# Builds @objectiveai/function-tree (tsup → dist/ with type declarations).
# Output is captured to .logs/build/objectiveai-function-tree.txt.
#
# Usage:
#   bash objectiveai-function-tree/build.sh

set -euo pipefail

MODULE="objectiveai-function-tree"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

if pnpm --filter @objectiveai/function-tree run build > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR"
  exit 1
fi
