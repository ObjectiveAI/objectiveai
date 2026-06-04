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

# tsup's dts worker needs more than Node's default heap to roll up
# declarations for the generated tree (~1800 zod modules incl. the
# cli command leaves) — without this it dies with
# ERR_WORKER_OUT_OF_MEMORY. This raises the JS heap only; it is
# unrelated to (and cannot raise) tsc's TS7056 declaration
# serialization cap, which is handled by #[json_schema_ignore] on the
# rust side.
export NODE_OPTIONS="--max-old-space-size=12288"

if pnpm --filter @objectiveai/sdk run build > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR"
  exit 1
fi
