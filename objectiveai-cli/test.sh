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

# Tear-down: clean up the api-spawn port file only.
#
# We deliberately do NOT wipe $RUNTIME_DIR on exit: keeping the
# staged tree around after a failed test makes post-mortem
# inspection possible (debug logs, per-test CONFIG_BASE_DIR
# contents, slotted binaries). The pre-test `rm -rf` below is
# the one and only place anything in this repo should wipe
# .objectiveai-tests/ — nothing in prepare.sh, the test code,
# or any other script should touch it.
#
# We also don't kill child processes here — leaked processes at
# script exit are a harness bug to fix, not papered over.
PORT_FILE=""
cleanup() {
  if [ -n "$PORT_FILE" ]; then
    rm -f "$PORT_FILE"
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

# Pre-run sweep: the cli `mem::forget`s the postgres handle so the
# postmaster daemonizes (its design — subsequent cli invocations
# in the same `CONFIG_BASE_DIR` reuse the live socket). For tests
# each per-test config_base_dir gets its own postmaster, and they
# all outlive the test process. Plugin fixtures spawned by the
# `agents spawn`-style tests can also outlive their parent cli if
# the test errored mid-stream. None of those leaks are this
# script's *fault*, but their open file handles block the
# `rm -rf` below.
#
# Scope: only processes whose binary lives inside $RUNTIME_DIR.
# Every leaked process from a prior test run launches from a
# slotted directory under that path (cli binary, plugin
# fixtures, postgres _shared-db-bin/), so an ExecutablePath
# prefix match identifies ours without touching anything else.
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

# Spawn the test api server in parallel — unless a parent harness has
# already provided `OBJECTIVEAI_TEST_PORT` (e.g. the root `test.sh`
# shares one server across all suites).
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  PORT_FILE="$(mktemp)"
  bash "$REPO_ROOT/test-spawn-api-server.sh" > "$PORT_FILE" 2>>"$LOG_FILE" &
  SPAWN_PID=$!
  wait "$SPAWN_PID"
  # test-spawn-api-server.sh prints "<port> <pid>"; we only need the
  # port. The api server keeps running in the background and is
  # expected to clean itself up (or be reaped by a parent harness).
  read -r PORT _ < "$PORT_FILE"
  rm -f "$PORT_FILE"
  PORT_FILE=""
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

wait "$PREP_PID"

# Run tests, capture all output. cargo-nextest is installed locally by
# `build-bin.sh` into `bin/` — see the [workspace.metadata.tools] table
# in the root Cargo.toml. Output is appended to $LOG_FILE so any
# prepare-step or api-spawn errors that landed there survive.
if "$NEXTEST" nextest run --manifest-path "$SCRIPT_DIR/Cargo.toml" "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
  PASSED=$(sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  FAILED=$(sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  TOTAL=$((PASSED + FAILED))
  echo "$MODULE: PASS $PASSED/$TOTAL"
else
  PASSED=$(sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  FAILED=$(sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  TOTAL=$((PASSED + FAILED))
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
