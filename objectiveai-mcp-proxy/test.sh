#!/usr/bin/env bash
# Runs the MCP proxy compliance suite: rmcp-driven Rust integration
# tests AND @modelcontextprotocol/sdk-driven TypeScript tests, both
# against the same proxy + test-upstream subprocesses.
#
# Output is captured to .logs/test/objectiveai-mcp-proxy.txt.
# Prints exactly one line: "$MODULE: PASS N/N" or "$MODULE: FAIL N/N".
#
# Usage:
#   bash objectiveai-mcp-proxy/test.sh

set -euo pipefail

MODULE="objectiveai-mcp-proxy"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

run_all() {
  # `set -e` is automatically disabled inside a function called in an
  # `|| EXIT=$?` context, so we can't rely on early steps short-
  # circuiting the function. Track exit status explicitly per step and
  # OR them together so a failing nextest run isn't masked by a passing
  # pnpm run (or vice versa).
  local rc=0
  echo "==> Building test binaries"
  cargo build \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p objectiveai-mcp-proxy \
    -p test-upstream || rc=$?
  # cargo-nextest is installed locally by `build-bin.sh` into `bin/` —
  # see the [workspace.metadata.tools] table in the root Cargo.toml.
  echo "==> Rust integration tests (rmcp client → proxy)"
  "$NEXTEST" nextest run \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p objectiveai-mcp-proxy \
    --tests || rc=$?
  echo "==> TypeScript integration tests (@modelcontextprotocol/sdk → proxy)"
  pnpm --filter tests-ts install || rc=$?
  pnpm --filter tests-ts run test || rc=$?
  return "$rc"
}

EXIT=0
run_all >> "$LOG_FILE" 2>&1 || EXIT=$?

# Strip ANSI escape codes once for vitest counting; cargo output is
# usually plain when redirected, but stripping is harmless either way.
CLEAN=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG_FILE")

# Nextest: `   Summary [  59.27s] 29 tests run: 29 passed, 0 skipped`
# (`N failed` appears in the same list on red runs). awk only — a grep
# with zero matches would exit 1 and kill the script under pipefail.
RUST_PASSED=$(echo "$CLEAN" | awk \
  '/tests run:/ { for (i = 2; i <= NF; i++) if ($i ~ /^passed/) s += $(i-1) }
   END { print s + 0 }')
RUST_FAILED=$(echo "$CLEAN" | awk \
  '/tests run:/ { for (i = 2; i <= NF; i++) if ($i ~ /^failed/) s += $(i-1) }
   END { print s + 0 }')

# Vitest: final summary line `     Tests  18 passed (18)` (with the
# `Test Files` line just above it being similar — narrow to lines
# starting with whitespace + `Tests ` exactly; on red runs it reads
# `Tests  1 failed | 17 passed (18)`).
TS_PASSED=$(echo "$CLEAN" | awk \
  '/^[[:space:]]*Tests[[:space:]]/ { for (i = 2; i <= NF; i++) if ($i ~ /^passed/) p = $(i-1) }
   END { print p + 0 }')
TS_FAILED=$(echo "$CLEAN" | awk \
  '/^[[:space:]]*Tests[[:space:]]/ { for (i = 2; i <= NF; i++) if ($i ~ /^failed/) f = $(i-1) }
   END { print f + 0 }')

PASSED=$(( RUST_PASSED + ${TS_PASSED:-0} ))
FAILED=$(( RUST_FAILED + ${TS_FAILED:-0} ))
TOTAL=$(( PASSED + FAILED ))

if [ "$EXIT" -eq 0 ]; then
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: PASS $PASSED/$TOTAL"
  else
    echo "$MODULE: PASS"
  fi
else
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
