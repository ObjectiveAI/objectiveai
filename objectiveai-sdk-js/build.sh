#!/usr/bin/env bash
# Builds objectiveai-sdk-js.
# Output is captured to .logs/build/objectiveai-sdk-js.txt.
#
# Usage:
#   bash objectiveai-sdk-js/build.sh

set -euo pipefail

MODULE="objectiveai-sdk-js"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

if pnpm --filter @objectiveai/sdk run build > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR"
  exit 1
fi
