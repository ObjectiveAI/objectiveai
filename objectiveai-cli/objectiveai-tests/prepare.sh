#!/usr/bin/env bash
# Builds the cli + every test fixture binary and slots each into the
# shared test home (`home/bin/...`). Removes itself on success.
# Multi-platform: the `.exe` suffix is detected from the produced cli
# binary, not from $OSTYPE heuristics.
#
# Every test shares ONE `OBJECTIVEAI_DIR` (the `home/` dir staged
# here) and isolates itself with `OBJECTIVEAI_STATE=<test-fn-name>`
# (per-test `home/state/<test>/`) — including its OWN postmaster on
# its own port (`db spawn` per test). Only `home/bin` is shared:
# fixture plugins/tools (coordinates are distinct across tests, and
# the one shared binary — count-tool — keys its counter files by MCP
# session id), the objectiveai-db vehicle, and the postgres install
# at `home/bin/pg-bin/`, which this script pre-warms via INSTALL_ONLY
# so per-test spawns only pay initdb+start.
#
# Run once per checkout — the deposited binaries are gitignored and
# the script itself self-removes from the working tree (the committed
# copy reappears on the next `git checkout` / `git pull`).
#
# Path layout is computed from the script's own location, so this
# works regardless of the caller's working directory.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"             # objectiveai-tests/
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"             # workspace root
BIN="$ROOT/home/bin"
# Dedicated target dir for the cli binary only. The cli build uses
# `--no-default-features --features rustpython`, which produces a
# differently-featured `objectiveai-cli` artifact than the default
# `target/debug` (which nextest populates when it compiles the
# integration tests linking against `objectiveai-cli` as a lib).
# Co-locating those two builds would force-rebuild the cli lib on
# every flip between prepare.sh and nextest, so the cli build keeps
# its own slot. Sub-crates (SDK, mcp, fixtures) compile under default
# features and CAN share the workspace target.
CLI_TARGET_DIR="$REPO_ROOT/target/objectiveai-tests"

# Two concurrent cargo invocations:
#   - The cli build uses CLI_TARGET_DIR so the rustpython-featured
#     artifact stays isolated from nextest's default-featured one.
#   - The fixture builds use the workspace's default target/, so the
#     shared dep tree (objectiveai-sdk, objectiveai-mcp, etc.) is
#     compiled exactly once and reused by nextest.
(cargo build \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p objectiveai-cli \
    --no-default-features --features rustpython \
    --target-dir "$CLI_TARGET_DIR") &
PID_CLI=$!

(cargo build \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p hello-tool -p error-tool -p count-tool \
    -p hello-plugin -p test-mcp-plugin \
    -p test-mcp-plugin-named -p test-mcp-plugin-foo-headers \
    -p objectiveai-db) &
PID_FIX=$!

wait "$PID_CLI" "$PID_FIX"

CLI_BIN_DIR="$CLI_TARGET_DIR/debug"
FIX_BIN_DIR="$REPO_ROOT/target/debug"
if [ -f "$CLI_BIN_DIR/objectiveai-cli.exe" ]; then EXE=".exe"; else EXE=""; fi

slot() {
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2"
}

# Layout: every plugin/tool lives at
# `home/bin/{plugins,tools}/<owner>/<name>/<version>/`. Plugin
# binaries are `plugin[.exe]`; tool binaries keep their exec base name
# (the manifest's per-OS `exec` invokes `./<name>` from that version
# dir, which is the CWD at run time). objectiveai-db sits directly in
# `home/bin/` where the cli's `db spawn` resolves it.
slot "$CLI_BIN_DIR/objectiveai-cli$EXE"               "$ROOT/objectiveai-cli$EXE" &
slot "$FIX_BIN_DIR/objectiveai-db$EXE"                "$BIN/objectiveai-db$EXE" &
slot "$FIX_BIN_DIR/test-mcp-plugin$EXE"               "$BIN/plugins/testorg/test-mcp-plugin/1.0.0/plugin$EXE" &
slot "$FIX_BIN_DIR/hello-plugin$EXE"                  "$BIN/plugins/objectiveai/hello/0.0.1/plugin$EXE" &
slot "$FIX_BIN_DIR/hello-tool$EXE"                    "$BIN/tools/objectiveai/hello/0.0.1/hello-tool$EXE" &
slot "$FIX_BIN_DIR/error-tool$EXE"                    "$BIN/tools/objectiveai/error/0.0.1/error-tool$EXE" &
slot "$FIX_BIN_DIR/test-mcp-plugin-foo-headers$EXE"   "$BIN/plugins/testorg/test-mcp-plugin-foo-headers/1.0.0/plugin$EXE" &

for n in 0 1 2 3 4 5 6 7 8 9; do
  slot "$FIX_BIN_DIR/count-tool$EXE" \
       "$BIN/tools/testorg/tool$n/1.0.0/count-tool$EXE" &
done

for name in dup-alpha dup-bravo dup-charlie dup-delta dup-echo \
            same-alpha same-bravo same-charlie same-delta same-echo; do
  slot "$FIX_BIN_DIR/test-mcp-plugin-named$EXE" \
       "$BIN/plugins/testorg/$name/1.0.0/plugin$EXE" &
done

wait

# Pre-warm the shared postgres install (`home/bin/pg-bin/`): the
# ~163M extract happens exactly once, here, with bounded retries (a
# real-time AV/indexer scanning freshly-written executables can hold
# transient handles and fail the extract mid-write — the race is
# timing-only, a clean retry succeeds). objectiveai-db's own
# install-lock + completion marker make the wipe/retry safe. Per-test
# `db spawn`s then only pay initdb+start for their state's cluster.
pg_installed=""
for attempt in 1 2 3 4 5; do
  if OBJECTIVEAI_DIR="$ROOT/home" INSTALL_ONLY=1 "$BIN/objectiveai-db$EXE"; then
    pg_installed=1
    break
  fi
  echo "prepare.sh: postgres install attempt $attempt did not complete; retrying" >&2
  rm -rf "$BIN/pg-bin"
  sleep "$attempt"   # 1s, 2s, 3s, 4s — widening grace for the scanner
done
if [ -z "$pg_installed" ]; then
  echo "prepare.sh: FATAL — postgres install failed after 5 attempts" >&2
  exit 1
fi

rm -- "$0"
