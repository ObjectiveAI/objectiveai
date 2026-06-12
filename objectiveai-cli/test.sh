#!/usr/bin/env bash
# Runs objectiveai-cli tests.
# Output is captured to .logs/test/objectiveai-cli.txt.
#
# Usage:
#   bash objectiveai-cli/test.sh
#   bash objectiveai-cli/test.sh -- --test-threads=1   # pass args to nextest

set -euo pipefail

MODULE="objectiveai-cli"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"
RUNTIME_DIR="$SCRIPT_DIR/.objectiveai-tests"

mkdir -p "$LOG_DIR"
: > "$LOG_FILE"

# Tear-down: reap the api server we spawned (if any).
#
# Mirrors the root `test.sh`: when this suite spawns its OWN api
# server (no `OBJECTIVEAI_TEST_PORT` inherited from a parent
# harness), it owns that server's lifetime and must kill it on
# exit — otherwise every standalone run leaks one long-lived
# `objectiveai-api` process (the server never self-terminates,
# and the pre-run sweep can't catch it because it runs from
# `target/`, not `$RUNTIME_DIR`). `SERVER_PID` stays empty when
# we inherit a parent's port, so the trap leaves the parent's
# server alone.
#
# We deliberately do NOT wipe $RUNTIME_DIR on exit: keeping the
# staged tree around after a failed test makes post-mortem
# inspection possible (debug logs, per-test CONFIG_BASE_DIR
# contents, slotted binaries). The pre-test `rm -rf` below is
# the one and only place anything in this repo should wipe
# .objectiveai-tests/ — nothing in prepare.sh, the test code,
# or any other script should touch it.
SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# Parse flags
CARGO_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; CARGO_ARGS=("$@"); break ;;
    *)  CARGO_ARGS+=("$1"); shift ;;
  esac
done

# Pre-run sweep: plugin fixtures spawned by the `agents spawn`-style
# tests can outlive their parent cli if the test errored mid-stream,
# and their open file handles block the `rm -rf` below.
#
# Scope: only processes whose binary lives inside $RUNTIME_DIR.
# Every leaked process from a prior test run launches from a
# slotted directory under that path (cli binary, plugin
# fixtures), so an ExecutablePath prefix match identifies ours
# without touching anything else.
# We deliberately do NOT match on command-line substring — that
# was the old behavior and it killed unrelated processes on the
# system that happened to share a substring. See the in-script
# comment near `cleanup()` for why we sweep here and not at exit.
if command -v powershell.exe >/dev/null 2>&1; then
  if command -v cygpath >/dev/null 2>&1; then
    SWEEP_PATH="$(cygpath -w "$RUNTIME_DIR")"
  else
    SWEEP_PATH="$RUNTIME_DIR"
  fi
  OAI_SWEEP_PATH="$SWEEP_PATH" powershell.exe -NoProfile -Command '
    $target = [System.IO.Path]::GetFullPath($env:OAI_SWEEP_PATH).TrimEnd([char]"\") + [char]"\";
    Get-CimInstance Win32_Process | Where-Object {
      $_.ExecutablePath -ne $null -and
      $_.ExecutablePath.StartsWith($target, [System.StringComparison]::OrdinalIgnoreCase)
    } | ForEach-Object {
      try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop } catch {}
    }
  ' >/dev/null 2>&1 || true
elif command -v lsof >/dev/null 2>&1; then
  # `lsof +D` recursively lists every PID holding a file open
  # under the given directory tree. We only want PIDs, dedupe
  # them, then SIGKILL.
  lsof -t +D "$RUNTIME_DIR" 2>/dev/null | sort -u | while read -r pid; do
    [ -n "$pid" ] && kill -9 "$pid" 2>/dev/null || true
  done
elif command -v fuser >/dev/null 2>&1; then
  fuser -k "$RUNTIME_DIR" >/dev/null 2>&1 || true
fi

# Stage the runtime test tree: a fresh copy of the committed
# `objectiveai-tests/` source. The copied tree includes a self-
# removing `prepare.sh` which cargo-builds the cli + every fixture
# crate and slots binaries into the right per-test directories.
#
# Fail FAST and LOUD if the wipe can't complete — but give killed
# processes a moment to die first. The pre-run sweep above fires
# `Stop-Process -Force` (or SIGKILL) and returns immediately, yet
# Windows does not release a terminated postmaster's open file
# handles synchronously. The very next `rm -rf` can therefore race
# the kernel's handle teardown and fail with "Device or resource
# busy" on $RUNTIME_DIR itself, even though nothing will hold it a
# second later. So retry with a short backoff; only if it STILL
# can't wipe after the grace window do we treat it as a genuinely
# stuck handle (a leaked process the sweep's exe-path filter
# missed) and abort with a diagnostic instead of a silent
# `set -e` exit.
RMRF_ERR="$(mktemp)"
wiped=""
for attempt in 1 2 3 4 5; do
  if rm -rf "$RUNTIME_DIR" 2>"$RMRF_ERR"; then
    wiped=1
    break
  fi
  sleep "$attempt"   # 1s, 2s, 3s, 4s — widening grace for handle release
done
if [ -z "$wiped" ]; then
  echo "$MODULE: FATAL — could not wipe runtime dir before staging" >&2
  echo "  path: $RUNTIME_DIR" >&2
  sed 's/^/  rm: /' "$RMRF_ERR" >&2 || true
  rm -f "$RMRF_ERR"
  echo "  A process may still be alive inside that folder, holding a" >&2
  echo "  handle open and preventing deletion. Kill it, then re-run." >&2
  exit 1
fi
rm -f "$RMRF_ERR"
cp -R "$SCRIPT_DIR/objectiveai-tests" "$RUNTIME_DIR"
bash "$RUNTIME_DIR/prepare.sh" >>"$LOG_FILE" 2>&1 &
PREP_PID=$!

# Spawn ONE test api server — unless a parent harness has already
# provided `OBJECTIVEAI_TEST_PORT` (e.g. the root `test.sh` shares a
# single server across all suites), in which case we inherit it and
# spawn nothing.
#
# `test-spawn-api-server.sh` prints "<port> <pid>" once the server
# is accepting connections, then returns while the server keeps
# running in the background. We capture BOTH: the port to point the
# tests at, and the pid so the cleanup trap can reap our server on
# exit. Same `read … < <(…)` shape the root `test.sh` uses. prepare.sh
# is already building in the background (PREP_PID), so this readiness
# wait overlaps with staging.
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  read -r PORT SERVER_PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>>"$LOG_FILE") || {
    echo "$MODULE: FATAL — failed to spawn API server (see $LOG_FILE)" >&2
    exit 1
  }
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

wait "$PREP_PID"

# Run tests, capture all output. cargo-nextest is installed locally by
# `build-bin.sh` into `bin/` — see the [workspace.metadata.tools] table
# in the root Cargo.toml. Output is appended to $LOG_FILE so any
# prepare-step or api-spawn errors that landed there survive.
if "$NEXTEST" nextest run --manifest-path "$SCRIPT_DIR/Cargo.toml" "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
  NEXTEST_OK=1
else
  NEXTEST_OK=0
fi

# Parse counts from nextest's own summary line ONLY, e.g.
#   "Summary [ 72.9s] 146 tests run: 146 passed (16 leaky), 0 skipped"
#   "Summary [ ... ] 148 tests run: 146 passed, 2 failed, 0 skipped"
# Anchoring on the "N tests run:" line is essential: the rest of
# $LOG_FILE carries cargo build output and prepare.sh diagnostics
# (e.g. a postgres-install retry note) that also contain the words
# "passed"/"failed" and would otherwise be miscounted as test
# results. `tail -1` takes the final run's summary; `|| true` keeps
# `set -o pipefail` from aborting when grep finds no match.
SUMMARY_LINE="$(grep -E '[0-9]+ tests run:' "$LOG_FILE" | tail -1 || true)"
PASSED=$(printf '%s\n' "$SUMMARY_LINE" | sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p')
FAILED=$(printf '%s\n' "$SUMMARY_LINE" | sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p')
PASSED=${PASSED:-0}
FAILED=${FAILED:-0}
TOTAL=$((PASSED + FAILED))

if [ "$NEXTEST_OK" -eq 1 ]; then
  echo "$MODULE: PASS $PASSED/$TOTAL"
else
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
