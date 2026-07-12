#!/usr/bin/env bash
# test-integration.sh — the integration suite.
#
# Owns the shared API server: every integration suite talks to ONE server.
# This script resets the shared .objectiveai test root, (re)installs the
# cli/api binaries from the kept release zip, configures them, spawns the
# server, exports OBJECTIVEAI_ADDRESS, installs the plugin/tool fixtures,
# then runs — all in parallel —
#   - the Rust integration crates (objectiveai-api-tests, objectiveai-cli-tests)
#     via host cargo-nextest, and
#   - the SDK importer projects (go/py/js) under tests/objectiveai-sdk-*-tests,
#     which import the built SDKs and drive the same server.
# The server is killed on exit (trap). The unit suite (test-unit.sh) and the
# SDK unit suite (test-sdk.sh) need no server, so they no longer share this
# setup — it moved here from test.sh, since only this suite needs it.
#
# Uses the host cargo-nextest — whatever `cargo nextest` resolves to on PATH.
# The SDK importer projects assume `bash build.sh` already prepared them
# (pnpm workspace link / go module graph / py venv).
#
# Exit status: 0 iff every run exited 0; 1 if any failed.
#
# Usage:
#   bash test-integration.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
OAI_DIR="$REPO_ROOT/.objectiveai"
LOG_DIR="$REPO_ROOT/.logs/tests"
mkdir -p "$LOG_DIR"

# Host nextest (whatever `cargo nextest` resolves to on PATH).
if ! cargo nextest --version >/dev/null 2>&1; then
  echo "test-integration: host cargo-nextest not found on PATH; install it (cargo install cargo-nextest)" >&2
  exit 1
fi

PY="$(command -v python3 || command -v python || true)"
[ -n "$PY" ] || { echo "test-integration: need python to enumerate workspace crates" >&2; exit 1; }

# One timestamp for the whole run, so a run's logs sort together.
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

# The installed cli binary (.exe on Windows). May not exist yet on a fresh tree.
oai_bin() {
  if [ -f "$OAI_DIR/bin/objectiveai.exe" ]; then
    printf '%s\n' "$OAI_DIR/bin/objectiveai.exe"
  else
    printf '%s\n' "$OAI_DIR/bin/objectiveai"
  fi
}

# ── Server teardown on exit ─────────────────────────────────────────
cleanup() {
  local b
  b="$(oai_bin)"
  if [ -f "$b" ]; then
    echo "test-integration: kill-all (post)"
    OBJECTIVEAI_DIR="$OAI_DIR" "$b" kill-all || true
  fi
}
trap cleanup EXIT

# ── Step 1: kill-all, only if the cli is already installed ──────────
BIN="$(oai_bin)"
if [ -f "$BIN" ]; then
  echo "test-integration: kill-all (pre)"
  OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" kill-all || true
fi

# ── Step 2: reset .objectiveai down to the keepers ──────────────────
# In bin/, keep only top-level *.zip release assets and the pg-bin postgres
# dir; delete everything else (files + folders, recursively). Wipe state/.
if [ -d "$OAI_DIR/bin" ]; then
  find "$OAI_DIR/bin" -mindepth 1 -maxdepth 1 \
    ! -name '*.zip' ! -name 'pg-bin' -exec rm -rf {} +
fi
rm -rf "$OAI_DIR/state"

# ── Step 3: (re)install the binaries from the kept zip ──────────────
if ! bash "$REPO_ROOT/install.sh" --objectiveai-dir "$OAI_DIR" --no-export-path; then
  echo "test-integration: install.sh failed" >&2
  exit 1
fi

# ── Step 4: configure for testing ───────────────────────────────────
BIN="$(oai_bin)"
OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" api config mcp-call-timeout-ms set --value 300000 --global \
  || { echo "test-integration: 'api config mcp-call-timeout-ms set' failed" >&2; exit 1; }
OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" api config backoff-max-elapsed-time-ms set --value 0 --global \
  || { echo "test-integration: 'api config backoff-max-elapsed-time-ms set' failed" >&2; exit 1; }

# ── Step 5: spawn the api server and publish its address ────────────
# Every integration suite (the Rust crates AND the SDK importers) talks to
# this one server. Spawn it (idempotent behind the api lockfile singleton)
# and export its published base URL as OBJECTIVEAI_ADDRESS — the same env var
# the objectiveai client reads — so every child suite inherits it.
SPAWN_OUT="$(OBJECTIVEAI_DIR="$OAI_DIR" "$BIN" api spawn)" \
  || { echo "test-integration: 'api spawn' failed" >&2; printf '%s\n' "$SPAWN_OUT" >&2; exit 1; }
OBJECTIVEAI_ADDRESS="$(printf '%s' "$SPAWN_OUT" | grep -oE 'https?://[^"]+' | head -1)"
if [ -z "$OBJECTIVEAI_ADDRESS" ]; then
  echo "test-integration: could not parse a URL from 'api spawn' output:" >&2
  printf '%s\n' "$SPAWN_OUT" >&2
  exit 1
fi
export OBJECTIVEAI_ADDRESS
echo "test-integration: api server at $OBJECTIVEAI_ADDRESS"

# ── Step 6: install the plugin/tool fixtures ────────────────────────
# The Rust integration tests exec these. Run every install.sh found under
# tests/plugins and tests/tools in parallel; abort if any fails.
ipids=()
inames=()
while IFS= read -r installer; do
  iname="$(basename "$(dirname "$installer")")"
  bash "$installer" >"$LOG_DIR/install-${iname}-${TIMESTAMP}.txt" 2>&1 &
  ipids+=("$!")
  inames+=("$iname")
done < <(find "$REPO_ROOT/tests/plugins" "$REPO_ROOT/tests/tools" -name install.sh 2>/dev/null | sort)

ifailed=0
for i in "${!ipids[@]}"; do
  if wait "${ipids[$i]}"; then
    echo "test-integration: fixture install ${inames[$i]}: OK"
  else
    echo "test-integration: fixture install ${inames[$i]}: FAILED" >&2
    ifailed=1
  fi
done
if [ "$ifailed" -ne 0 ]; then
  echo "test-integration: one or more fixture installs failed; aborting" >&2
  exit 1
fi

# ── Step 7: Rust integration crates — discover + prebuild ───────────
# Workspace member crates whose manifest lives under one of the two
# integration-test crate dirs.
mapfile -t CRATES < <(
  cargo metadata --no-deps --format-version 1 --manifest-path "$REPO_ROOT/Cargo.toml" \
  | "$PY" -c '
import json, sys
md = json.load(sys.stdin)
root = md["workspace_root"].replace("\\", "/")
integ = (root + "/tests/objectiveai-api-tests/", root + "/tests/objectiveai-cli-tests/")
members = set(md["workspace_members"])
names = set()
for p in md["packages"]:
    if p["id"] not in members:
        continue
    manifest = p["manifest_path"].replace("\\", "/")
    if not any(manifest.startswith(d) for d in integ):
        continue
    names.add(p["name"])
for n in sorted(names):
    print(n)
' | tr -d '\r'
)

# Build each crate's test binaries up front, ONE AT A TIME, so the parallel
# run phase below only executes (not rebuilds concurrently against the
# shared target dir). Every crate is attempted so one failure doesn't hide
# the rest.
BUILD_LOG_DIR="$REPO_ROOT/.logs/build"
mkdir -p "$BUILD_LOG_DIR"
prebuild_failed=0
for crate in ${CRATES[@]+"${CRATES[@]}"}; do
  echo "test-integration: build $crate ..."
  if ! cargo nextest run --no-run --manifest-path "$REPO_ROOT/Cargo.toml" -p "$crate" \
       >"$BUILD_LOG_DIR/${crate}-integration-nextest-${TIMESTAMP}.txt" 2>&1; then
    echo "test-integration: BUILD FAILED: $crate (see .logs/build/${crate}-integration-nextest-${TIMESTAMP}.txt)" >&2
    prebuild_failed=1
  fi
done
if [ "$prebuild_failed" -ne 0 ]; then
  echo "test-integration: one or more test builds failed; aborting" >&2
  exit 1
fi

# ── Step 8: run all integration suites in parallel ──────────────────
# The Rust crates (one nextest run each) plus the three SDK importer
# projects (go/py/js), which import the built SDKs and hit the same server.
# No ordering between them.
pids=()
names=()
launch() {  # launch <name> <command...>
  local name="$1"; shift
  local log="$LOG_DIR/${name}-integration-${TIMESTAMP}.txt"
  ( "$@" ) >"$log" 2>&1 &
  pids+=("$!")
  names+=("$name")
}

for crate in ${CRATES[@]+"${CRATES[@]}"}; do
  launch "$crate" cargo nextest run --no-tests=pass --manifest-path "$REPO_ROOT/Cargo.toml" -p "$crate"
done

# SDK importer projects. OBJECTIVEAI_ADDRESS is exported above; each suite's
# tests skip themselves only when it is unset (it is set here).
launch sdk-go bash "$REPO_ROOT/tests/objectiveai-sdk-go-tests/test.sh"
launch sdk-py bash "$REPO_ROOT/tests/objectiveai-sdk-py-tests/test.sh"
launch sdk-js bash "$REPO_ROOT/tests/objectiveai-sdk-js-tests/test.sh"

failed=0
for i in "${!pids[@]}"; do
  if wait "${pids[$i]}"; then
    echo "test-integration: ${names[$i]}: PASS"
  else
    echo "test-integration: ${names[$i]}: FAIL"
    failed=1
  fi
done

exit "$failed"
