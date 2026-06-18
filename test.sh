#!/usr/bin/env bash
# test.sh — full test orchestration.
#
# Resets the shared .objectiveai test root, (re)installs the cli/api
# binaries from the kept release zip, configures them for testing, then
# runs the unit / sdk / integration suites in parallel and reports their
# aggregate result.
#
# Flags (all optional): --no-unit, --no-sdk, --no-integration.
#
# Special case: with BOTH --no-sdk and --no-integration, none of the
# .objectiveai / server setup is needed (unit tests hit no server), so
# we skip all of it and defer straight to test-unit.sh.
#
# Usage:
#   bash test.sh [--no-unit] [--no-sdk] [--no-integration]

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
OAI_DIR="$REPO_ROOT/.objectiveai"
USAGE="Usage: bash test.sh [--no-unit] [--no-sdk] [--no-integration]"

NO_UNIT=0
NO_SDK=0
NO_INTEGRATION=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --no-unit)        NO_UNIT=1; shift ;;
    --no-sdk)         NO_SDK=1; shift ;;
    --no-integration) NO_INTEGRATION=1; shift ;;
    -h|--help)        echo "$USAGE"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; echo "$USAGE" >&2; exit 1 ;;
  esac
done

# The installed cli binary (.exe on Windows). May not exist yet on a
# fresh tree.
oai_bin() {
  if [ -f "$OAI_DIR/bin/objectiveai.exe" ]; then
    printf '%s\n' "$OAI_DIR/bin/objectiveai.exe"
  else
    printf '%s\n' "$OAI_DIR/bin/objectiveai"
  fi
}

# ── Unit-only shortcut ──────────────────────────────────────────────
# With both the SDK and integration suites disabled, only unit tests can
# run, and they need no server / .objectiveai setup — skip everything
# else and defer to test-unit.sh.
if [ "$NO_SDK" = "1" ] && [ "$NO_INTEGRATION" = "1" ]; then
  if [ "$NO_UNIT" = "1" ]; then
    echo "test: nothing to run (--no-unit --no-sdk --no-integration)"
    exit 0
  fi
  exec bash "$REPO_ROOT/test-unit.sh"
fi

# ── Step 1: kill-all, only if the cli is already installed ──────────
BIN="$(oai_bin)"
if [ -f "$BIN" ]; then
  echo "test: kill-all (pre)"
  OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" kill-all || true
fi

# ── Step 2: reset .objectiveai down to the keepers ──────────────────
# In bin/, keep only top-level *.zip release assets and the pg-bin
# postgres dir; delete everything else (files + folders, recursively).
# Wipe state/ entirely.
if [ -d "$OAI_DIR/bin" ]; then
  find "$OAI_DIR/bin" -mindepth 1 -maxdepth 1 \
    ! -name '*.zip' ! -name 'pg-bin' -exec rm -rf {} +
fi
rm -rf "$OAI_DIR/state"

# ── Step 3: (re)install the binaries from the kept zip ──────────────
if ! bash "$REPO_ROOT/install.sh" --objectiveai-dir "$OAI_DIR" --no-export-path; then
  echo "test: install.sh failed" >&2
  exit 1
fi

# ── Step 4: configure for testing ───────────────────────────────────
BIN="$(oai_bin)"
OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" api config mcp-timeout-ms set 300_000 --global \
  || { echo "test: 'api config mcp-timeout-ms set' failed" >&2; exit 1; }
OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" api config backoff-max-elapsed-time-ms set 0 --global \
  || { echo "test: 'api config backoff-max-elapsed-time-ms set' failed" >&2; exit 1; }

# ── Step 5: spawn the api server and publish its address ────────────
# The sdk and integration suites all talk to ONE shared server. Spawn
# it (idempotent behind the api lockfile singleton) and export its
# published base URL as OBJECTIVEAI_ADDRESS — the same env var the
# objectiveai client reads for its address — so every suite inherits
# it.
SPAWN_OUT="$(OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" api spawn)" \
  || { echo "test: 'api spawn' failed" >&2; printf '%s\n' "$SPAWN_OUT" >&2; exit 1; }
OBJECTIVEAI_ADDRESS="$(printf '%s' "$SPAWN_OUT" | grep -oE 'https?://[^"]+' | head -1)"
if [ -z "$OBJECTIVEAI_ADDRESS" ]; then
  echo "test: could not parse a URL from 'api spawn' output:" >&2
  printf '%s\n' "$SPAWN_OUT" >&2
  exit 1
fi
export OBJECTIVEAI_ADDRESS
echo "test: api server at $OBJECTIVEAI_ADDRESS"

# ── Step 6: run the enabled suites in parallel ──────────────────────
pids=()
names=()
launch() { bash "$REPO_ROOT/$2" & pids+=("$!"); names+=("$1"); }
[ "$NO_UNIT" = "1" ]        || launch unit        test-unit.sh
[ "$NO_SDK" = "1" ]         || launch sdk         test-sdk.sh
[ "$NO_INTEGRATION" = "1" ] || launch integration test-integration.sh

failed=0
for i in "${!pids[@]}"; do
  if wait "${pids[$i]}"; then
    echo "test: ${names[$i]} suite: PASS"
  else
    echo "test: ${names[$i]} suite: FAIL"
    failed=1
  fi
done

# ── Step 7: kill-all again ──────────────────────────────────────────
BIN="$(oai_bin)"
if [ -f "$BIN" ]; then
  echo "test: kill-all (post)"
  OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" kill-all || true
fi

# Exit 0 iff every suite that ran exited 0.
exit "$failed"
