#!/usr/bin/env bash
# Builds the cli + every test fixture binary and slots each into the
# right per-test directory under this folder. Removes itself on
# success. Multi-platform: the `.exe` suffix is detected from the
# produced cli binary, not from $OSTYPE heuristics.
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
# Dedicated target dir for the cli binary only. The cli build uses
# `--no-default-features --features rustpython`, which produces a
# differently-featured `objectiveai-cli` artifact than the default
# `target/debug` (which nextest populates when it compiles the
# integration tests linking against `objectiveai-cli` as a lib).
# Co-locating those two builds would force-rebuild the cli lib on
# every flip between prepare.sh and nextest, so the cli build keeps
# its own slot. Sub-crates (SDK, mcp, fixtures) compile under
# default features and CAN share the workspace target.
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
    -p objectiveai-tests-pg-installer) &
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
# `<test>/{plugins,tools}/<owner>/<name>/<version>/`. Plugin binaries
# are `plugin[.exe]`; tool binaries keep their exec base name (the
# manifest's per-OS `exec` invokes `./<name>` from that version dir,
# which is the CWD at run time).
slot "$CLI_BIN_DIR/objectiveai-cli$EXE"               "$ROOT/objectiveai-cli$EXE" &
slot "$FIX_BIN_DIR/test-mcp-plugin$EXE"               "$ROOT/plugin_mcp_dispatch_round_trip/plugins/testorg/test-mcp-plugin/1.0.0/plugin$EXE" &
slot "$FIX_BIN_DIR/hello-plugin$EXE"                  "$ROOT/hello_plugin_dispatch_produces_expected_output/plugins/objectiveai/hello/0.0.1/plugin$EXE" &
slot "$FIX_BIN_DIR/hello-tool$EXE"                    "$ROOT/hello_tool_dispatch_snapshot/tools/objectiveai/hello/0.0.1/hello-tool$EXE" &
slot "$FIX_BIN_DIR/error-tool$EXE"                    "$ROOT/error_tool_dispatch_snapshot/tools/objectiveai/error/0.0.1/error-tool$EXE" &
slot "$FIX_BIN_DIR/test-mcp-plugin-foo-headers$EXE"   "$ROOT/function_swarm_writes_per_agent_files/plugins/testorg/test-mcp-plugin-foo-headers/1.0.0/plugin$EXE" &

for n in 0 1 2 3 4 5 6 7 8 9; do
  slot "$FIX_BIN_DIR/count-tool$EXE" \
       "$ROOT/two_agents_continuations_count_persists_per_session/tools/testorg/tool$n/1.0.0/count-tool$EXE" &
  slot "$FIX_BIN_DIR/count-tool$EXE" \
       "$ROOT/test_twenty_agents_json_schema_10x_tools_seed_42/tools/testorg/tool$n/1.0.0/count-tool$EXE" &
done

for name in dup-alpha dup-bravo dup-charlie dup-delta dup-echo; do
  slot "$FIX_BIN_DIR/test-mcp-plugin-named$EXE" \
       "$ROOT/duplicate_tool_names_routed_across_turns/plugins/testorg/$name/1.0.0/plugin$EXE" &
done

for name in same-alpha same-bravo same-charlie same-delta same-echo; do
  slot "$FIX_BIN_DIR/test-mcp-plugin-named$EXE" \
       "$ROOT/duplicate_server_names_routed_across_turns/plugins/testorg/$name/1.0.0/plugin$EXE" &
done

wait

# Slot the postgres installer alongside the cli binary, then run
# it to extract the bundled archive once into a shared dir.
# Symlink every committed test dir's `db-bin/` to that shared
# install so per-test cli children skip the ~163M extract on
# their `pg.setup()` call.
slot "$FIX_BIN_DIR/objectiveai-tests-pg-installer$EXE" \
     "$ROOT/objectiveai-tests-pg-installer$EXE"

SHARED="$ROOT/_shared-db-bin"

# Run the installer with bounded retries. The extract writes
# hundreds of postgres binaries/libraries; on some platforms a
# real-time AV/indexer scanning a freshly-written executable holds
# a transient handle on it, so the crate's extraction intermittently
# fails with an "Access is denied"-class error mid-write. The race
# is timing-only — a clean retry succeeds — so wipe the partial
# install (and its stale archive lock) and try again, hard-failing
# only if it can't extract after several attempts. On platforms
# without that interference the first attempt simply succeeds and
# the loop is a no-op.
pg_installed=""
for attempt in 1 2 3 4 5; do
  if "$ROOT/objectiveai-tests-pg-installer$EXE" "$SHARED"; then
    pg_installed=1
    break
  fi
  echo "prepare.sh: postgres install attempt $attempt did not complete; retrying" >&2
  rm -rf "$SHARED"
  sleep "$attempt"   # 1s, 2s, 3s, 4s — widening grace for the scanner
done
if [ -z "$pg_installed" ]; then
  echo "prepare.sh: FATAL — postgres install failed after 5 attempts" >&2
  exit 1
fi

# Loop over committed test dirs (everything not starting with
# `_`). The `*/` glob filters out files. `[ -e dst ]` skips
# dirs that already have a `db-bin/` from a prior partial run.
for d in "$ROOT"/*/; do
  name=$(basename "$d")
  [[ "$name" == _* ]] && continue
  [[ -e "$d/db-bin" ]] && continue
  case "$OSTYPE" in
    msys*|cygwin*|win32*)
      # Create a directory junction via PowerShell. A junction
      # needs no privilege (unlike a symbolic link) and is
      # equivalent for our read-only reuse of the shared install.
      #
      # We deliberately do NOT use `cmd //c "mklink /J ..."`: MSYS
      # rewrites the `/J` flag as if it were a POSIX path, so cmd
      # receives a garbled argument and mklink fails with a bogus
      # "The filename, directory name, or volume label syntax is
      # incorrect". Paths are handed to PowerShell through the
      # environment (not interpolated into the command string) so
      # backslashes and spaces survive the bash→PowerShell hop
      # untouched.
      OAI_LINK="$(cygpath -w "$d/db-bin")" \
      OAI_TARGET="$(cygpath -w "$SHARED")" \
        powershell.exe -NoProfile -Command \
          'New-Item -ItemType Junction -Path $env:OAI_LINK -Target $env:OAI_TARGET -ErrorAction Stop | Out-Null' \
          >/dev/null
      ;;
    *)
      ln -s "$SHARED" "$d/db-bin"
      ;;
  esac
done

rm -- "$0"
