#!/usr/bin/env bash
# Runs objectiveai-viewer tests.
# Output is captured to .logs/test/objectiveai-viewer.txt.
#
# No preparation and no server lifecycle: the `cli_command`
# integration test drives the cli shim from the repo's committed
# `.objectiveai` test root (a pointer to the pre-built
# `target/debug/objectiveai-cli` — `test-build.sh` builds it), and
# any servers the cli needs self-spawn behind lockfile singletons.
# `test-cleanup.sh` brackets the run unless the root test.sh owns
# the bracketing (OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT).
#
# Usage:
#   bash objectiveai-viewer/test.sh
#   bash objectiveai-viewer/test.sh -- --test-threads=1

set -euo pipefail

MODULE="objectiveai-viewer"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

mkdir -p "$LOG_DIR"
: > "$LOG_FILE"

if [ -z "${OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT:-}" ]; then
  # Standalone run: reset the shared test root and (re)build the shim
  # binaries, in parallel — cleanup kills every lockfile-owning
  # process first thing, so nothing is left running the binaries the
  # build may relink.
  bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 & _CLEANUP_PID=$!
  bash "$REPO_ROOT/test-build.sh" >>"$LOG_FILE" 2>&1 & _BUILD_PID=$!
  wait "$_CLEANUP_PID"
  wait "$_BUILD_PID"
  # Post-test cleanup is kill-only: processes die but `state/` survives
  # so the run's db can be re-spawned and inspected. (Pre-test cleanup
  # above ran without the env var, so it still wiped a stale tree.)
  trap 'OBJECTIVEAI_TEST_CLEANUP_KILL_ONLY=1 bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 || true' EXIT INT TERM
fi

# Parse flags
CARGO_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; CARGO_ARGS=("$@"); break ;;
    *)  CARGO_ARGS+=("$1"); shift ;;
  esac
done

# ── Front-end (JS) validation ──────────────────────────────────────
# The release builds the viewer with `tsc && vite build` (the package
# `build` script). Nothing in the test suite used to exercise that, so
# a viewer that no longer typechecks against the workspace SDK only
# blew up at release time (e.g. 2.1.3: stale `@objectiveai/sdk`
# imports). Run the same build here so it fails as a test instead.
# Assumes the workspace SDK is already built — the root build.sh does
# that before tests; a standalone run needs `bash build.sh` first.
# NODE_OPTIONS bumps the heap: `tsc` over the large generated SDK type
# surface OOMs under Node's default.
echo "Validating viewer front-end (tsc && vite build)..." >>"$LOG_FILE"
if NODE_OPTIONS="--max-old-space-size=8192" \
   pnpm --dir "$REPO_ROOT" --filter objectiveai-viewer run build >>"$LOG_FILE" 2>&1; then
  JS_OK=1
else
  JS_OK=0
fi

# --lib --tests: skip the bin target. Tauri's deps (tauri, windows,
# encoding_rs, objectiveai_mcp_proxy) can't be linked under cargo test's
# bin-target compile pass — cargo can't satisfy the bin's cdylib-flavoured
# linkage AND the rlib form the test pass would need. Only the library +
# integration tests need to run; main.rs is a thin shim with no
# `#[cfg(test)]` coverage worth keeping. Release builds use cargo build /
# tauri build and are unaffected.
#
# cargo-nextest is installed locally by `build-bin.sh` into `bin/` — see
# the [workspace.metadata.tools] table in the root Cargo.toml.
if "$NEXTEST" nextest run -p objectiveai-viewer --lib --tests "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
  RUST_OK=1
else
  RUST_OK=0
fi

PASSED=$(sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
FAILED=$(sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
TOTAL=$((PASSED + FAILED))

# The suite passes only if BOTH the front-end build and the Rust tests
# pass. Surface which side failed so the one-line summary is actionable.
if [ "$JS_OK" -eq 1 ] && [ "$RUST_OK" -eq 1 ]; then
  echo "$MODULE: PASS $PASSED/$TOTAL"
else
  FAILED_PARTS=""
  [ "$JS_OK" -ne 1 ] && FAILED_PARTS="js"
  [ "$RUST_OK" -ne 1 ] && FAILED_PARTS="${FAILED_PARTS:+$FAILED_PARTS+}rust"
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL ($FAILED_PARTS)"
  else
    echo "$MODULE: FAIL ($FAILED_PARTS)"
  fi
  exit 1
fi
