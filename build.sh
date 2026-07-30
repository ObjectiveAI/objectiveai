#!/usr/bin/env bash
# Builds all packages in dependency order with parallelism.
#
# Phase 1 (background, NOT awaited until the very end): the product binaries
#   — one cargo build of cli, api, db, mcp in a SINGLE invocation (shared
#   compile cache); the Tauri viewer via objectiveai-viewer/build.sh
#   (`tauri build`, which builds the frontend against the committed
#   workspace SDK and embeds it + the icon, then drops the binary in
#   objectiveai-viewer/embed/<profile>/); and the two PyInstaller SDK runners (claude
#   + codex). None need the phase-2 build tools or json schemas, so they
#   start immediately and run concurrently with everything below.
#   (build_bin's `cargo install` uses a throwaway target dir, so it doesn't
#   fight this build for the workspace target lock; the json-schema/cffi/pyo3
#   cargo steps do share it and serialize behind it — fine, they're quick
#   once they get it.)
# Phase 2 (parallel): build/dev tools (wasm-pack, maturin, into ./bin/)
#   + objectiveai-json-schema. Independent of each other; both
#   must finish before phases 3+4, which need the tools + the schemas.
# Phase 3 (parallel): objectiveai-sdk-rs-wasm-js + objectiveai-sdk-rs-cffi
# Phase 4 (parallel): objectiveai-sdk-js + objectiveai-sdk-py + objectiveai-sdk-go
#                     (objectiveai-sdk-py builds its bundled Rust extension via maturin)
#                     (objectiveai-dotnet is disconnected from the root build for now;
#                     run `bash objectiveai-dotnet/build.sh` directly if you need it.)
# Integration prep (after phases 2-4): prepare the SDK importer test projects
#   under tests/objectiveai-sdk-{go,py,js}-tests, which consume the built SDK
#   artifacts (pnpm workspace link for js, module graph for go, the py venv for
#   py). Ordered after the SDK build; under --no-sdk it runs against the
#   existing artifacts. Skipped by --no-test-integration.
# Final: wait for phase 1, then package the HOST platform's 9 binaries into
#        the same per-platform zip the release ships
#        (objectiveai-<version>-<os>-<arch>.zip) and drop it in <OBJECTIVEAI_DIR>/bin
#        so the installer / `objectiveai update` can use it locally. Host
#        only — not the other five platforms.
# The viewer is built with `tauri build --no-bundle` (a raw exe, no
# installer bundle) — the cli no longer embeds it; this build just stages
# it into the zip alongside the others.
#
# Build profile defaults to debug. Pass --release for optimized builds;
# this propagates (via OBJECTIVEAI_BUILD_RELEASE) to the cffi, wasm-js,
# and pyo3 builds, which compile debug otherwise.
#
# Pass --no-zip to skip phase 1 (the product-binary + runner compilation)
# AND the final packaging — useful when you only want the schemas/tools/SDKs
# and don't need the per-platform zip. --no-sdk is the inverse: it skips
# phases 2-4 (build/dev tools incl. nextest, json schema, wasm/cffi, and the
# JS/Py/Go SDKs), leaving only phase 1 + packaging. Passing both skips every
# phase — by construction, nothing happens.
#
# By default phase 1 ALSO compiles the integration-test fixture crates —
# the plugin stubs under tests/plugins/ that the cli
# integration tests build and exec. They're discovered by glob (no
# hardcoded list, so new fixtures are picked up automatically), built in
# the same cargo invocation as the product binaries, and never packaged
# (they're test inputs, not shipped artifacts). Pass --no-test-integration
# to skip them — the release does this, since it ships zips, not tests.
# These fixtures ride phase 1, so --no-zip already excludes them;
# --no-test-integration only matters on a run that IS building the zip.
#
# Pass --skip-viewer to leave the Tauri viewer out of an otherwise-normal
# build: phase 1 skips objectiveai-viewer/build.sh and packaging stages a
# zip WITHOUT objectiveai-viewer (an install then keeps whatever viewer
# binary it already has). Errors under --no-zip — the viewer is only ever
# built as part of the zip phase, so there is nothing to skip there.
#
# Usage:
#   bash build.sh [--release] [--no-zip] [--no-sdk] [--no-test-integration] [--skip-viewer]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# ── Build profile ───────────────────────────────────────────────────────
# --release → optimized. Exported as OBJECTIVEAI_BUILD_RELEASE so the
# sub-builds (cffi, wasm-js, pyo3) pick it up — run_phase launches them
# with no args, so the env var is how the profile reaches them.
RELEASE=0
NO_ZIP=0
NO_SDK=0
NO_TEST_INTEGRATION=0
SKIP_VIEWER=0
USAGE="Usage: bash build.sh [--release] [--no-zip] [--no-sdk] [--no-test-integration] [--skip-viewer]"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --release) RELEASE=1; shift ;;
    --no-zip)  NO_ZIP=1; shift ;;
    --no-sdk)  NO_SDK=1; shift ;;
    --no-test-integration) NO_TEST_INTEGRATION=1; shift ;;
    --skip-viewer) SKIP_VIEWER=1; shift ;;
    -h|--help) echo "$USAGE"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; echo "$USAGE" >&2; exit 1 ;;
  esac
done

# --skip-viewer only makes sense when the viewer would otherwise be built,
# and the viewer is only ever built as part of the zip phase.
if [ "$SKIP_VIEWER" = "1" ] && [ "$NO_ZIP" = "1" ]; then
  echo "--skip-viewer conflicts with --no-zip: the viewer is not built when the zip phase is skipped — nothing to skip" >&2
  exit 1
fi
VIEWER_PID=""
if [ "$RELEASE" = "1" ]; then
  export OBJECTIVEAI_BUILD_RELEASE=1
  echo "Build profile: release"
else
  echo "Build profile: debug (pass --release for optimized builds)"
fi
if [ "$NO_ZIP" = "1" ]; then
  echo "Skipping phase 1 (product binaries + runners) and packaging (--no-zip)."
fi
if [ "$NO_SDK" = "1" ]; then
  echo "Skipping phases 2-4 (build tools, json schema, wasm/cffi, SDKs) (--no-sdk)."
fi
if [ "$NO_TEST_INTEGRATION" = "1" ]; then
  echo "Skipping the integration-test fixture crates (--no-test-integration)."
fi

LOG_DIR="$REPO_ROOT/.logs/build"
mkdir -p "$LOG_DIR"
# Timestamp for the per-crate build logs (<crate>-<BUILD_TS>.txt), the
# same shape as the test logs.
BUILD_TS="$(date +%Y%m%d-%H%M%S)"

# PROFILE_FLAG is shared by the runners and the cargo build so the
# embed/<profile>/ runner path lines up with target/<profile>/ for packaging.
PROFILE_FLAG=""
[ "$RELEASE" = "1" ] && PROFILE_FLAG="--release"

# Run a phase: launch all given scripts in parallel, wait for all, fail if any failed.
run_phase() {
  local pids=()
  for script in "$@"; do
    bash "$REPO_ROOT/$script" &
    pids+=($!)
  done

  local failed=false
  for pid in "${pids[@]}"; do
    if ! wait "$pid"; then
      failed=true
    fi
  done

  if $failed; then
    exit 1
  fi
}

# Installs wasm-pack and maturin into ./bin/ using the versions pinned
# in [workspace.metadata.tools] in Cargo.toml. Runs in phase 2 (parallel
# with json-schema); phases 3 and 4 invoke these tools, so it must
# finish before them. Output captured to .logs/build/build-bin.txt.
# (Test runs use the host cargo-nextest, not a pinned ./bin/ copy.)
build_bin() {
  local WASM_PACK_VERSION MATURIN_VERSION BIN_DIR
  WASM_PACK_VERSION=$(sed -n 's/^wasm-pack *= *"\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml")
  MATURIN_VERSION=$(sed -n 's/^maturin *= *"\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml")

  [ -n "$WASM_PACK_VERSION" ] || { echo "ERROR: Could not read wasm-pack version from Cargo.toml" >&2; return 1; }
  [ -n "$MATURIN_VERSION" ] || { echo "ERROR: Could not read maturin version from Cargo.toml" >&2; return 1; }

  BIN_DIR="$REPO_ROOT/bin"

  install_if_needed() {
    local name="$1" version="$2"
    local bin="$BIN_DIR/$name"
    if [ -x "$bin" ]; then
      local installed
      installed=$("$bin" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
      if [ "$installed" = "$version" ]; then
        echo "$name $version already installed, skipping."
        return
      fi
    fi
    echo "Installing $name $version..."
    cargo install "$name" --version "$version" --locked --root "$REPO_ROOT"
  }

  install_if_needed wasm-pack "$WASM_PACK_VERSION"
  install_if_needed maturin "$MATURIN_VERSION"

  echo "Done. Tools at $BIN_DIR/"
}

# Print the cargo package name of every integration-test fixture crate
# — the plugin stubs under tests/plugins/ — one
# per line. Discovery is by glob over their Cargo.toml `name` fields, so
# a fixture added under that folder is co-built with no edit here.
# Prints nothing if the folder is absent (the glob is nullglob-guarded
# by the `-f` test).
discover_test_integration_crates() {
  local toml name
  for toml in "$REPO_ROOT"/tests/plugins/*/Cargo.toml; do
    [ -f "$toml" ] || continue
    name=$(sed -n 's/^name *= *"\(.*\)"/\1/p' "$toml" | head -1)
    [ -n "$name" ] && printf '%s\n' "$name"
  done
}

# ── Phase 1 (background; awaited at the very end) ───────────────────────
# The four CLI/server product crates in one cargo build (shared cache),
# the Tauri viewer via its own build.sh (`tauri build` — embeds the frontend
# + icon; the frontend resolves @objectiveai/sdk from the committed
# workspace dist, so no SDK build is needed here), and the two PyInstaller
# SDK runners. mcp-proxy is NOT built here — objectiveai-api
# consumes it in-process as a regular cargo path dep, folded into the api's
# own cargo build. Skipped entirely under --no-zip (these binaries exist
# only to be packaged; nothing in phases 2-4 depends on them).
if [ "$NO_ZIP" != "1" ]; then
  bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner/build.sh" $PROFILE_FLAG &
  CLAUDE_RUNNER_PID=$!
  bash "$REPO_ROOT/objectiveai-codex-sdk-runner/build.sh" $PROFILE_FLAG &
  CODEX_RUNNER_PID=$!

  # objectiveai-mcp-laboratory: a musl-linux binary injected into Docker lab
  # containers, bundled (arch-matched) into every platform zip. Two cases:
  #  - CI release: the `laboratory` job prebuilds the musl binary and the zip
  #    job pre-places it (the binary alone, with NO .fingerprint) — detected
  #    here and used as-is, never rebuilt, so the release runners never
  #    cross-compile.
  #  - From source (install.sh / build-and-test): no pre-placed binary, so
  #    build it via its own build.sh — native `cargo build` on Linux,
  #    `cargo zigbuild` cross-compile on mac/windows (toolchain required;
  #    fingerprint-cached, so an unchanged rebuild is skipped and fast).
  LAB_ARCH=$(uname -m)
  case "$LAB_ARCH" in amd64) LAB_ARCH=x86_64 ;; arm64) LAB_ARCH=aarch64 ;; esac
  LAB_PROFILE="debug"; [ "$RELEASE" = "1" ] && LAB_PROFILE="release"
  LAB_DIR="$REPO_ROOT/objectiveai-mcp-laboratory/embed/${LAB_ARCH}-unknown-linux-musl/$LAB_PROFILE"
  LAB_PID=""
  if [ -f "$LAB_DIR/objectiveai-mcp-laboratory" ] && [ ! -f "$LAB_DIR/.fingerprint" ]; then
    echo "build.sh: using pre-placed objectiveai-mcp-laboratory binary (no rebuild)."
  else
    bash "$REPO_ROOT/objectiveai-mcp-laboratory/build.sh" $PROFILE_FLAG &
    LAB_PID=$!
  fi

  # objectiveai-db-proxy: the other musl-linux binary copied into lab
  # containers — a Postgres-over-WebSocket conduit, so a plugin inside a
  # container reaches the database through a plain connection string
  # instead of a network leg that does not exist. Same two cases, same
  # pre-placed-binary detection as the laboratory above.
  DB_PROXY_DIR="$REPO_ROOT/objectiveai-db-proxy/embed/${LAB_ARCH}-unknown-linux-musl/$LAB_PROFILE"
  DB_PROXY_PID=""
  if [ -f "$DB_PROXY_DIR/objectiveai-db-proxy" ] && [ ! -f "$DB_PROXY_DIR/.fingerprint" ]; then
    echo "build.sh: using pre-placed objectiveai-db-proxy binary (no rebuild)."
  else
    bash "$REPO_ROOT/objectiveai-db-proxy/build.sh" $PROFILE_FLAG &
    DB_PROXY_PID=$!
  fi

  # The integration-test fixture crates (plugins + tools under tests/)
  # co-build with the product binaries — discovered by glob over their
  # Cargo.toml names. They are NOT staged into the zip (test inputs, not
  # shipped). Default-on; --no-test-integration drops them (the release
  # uses that). They ride phase 1, so --no-zip already excludes them.
  FIXTURE_CRATES=()
  if [ "$NO_TEST_INTEGRATION" != "1" ]; then
    while IFS= read -r _fixture; do
      FIXTURE_CRATES+=("$_fixture")
    done < <(discover_test_integration_crates)
    if [ "${#FIXTURE_CRATES[@]}" -gt 0 ]; then
      echo "Co-building ${#FIXTURE_CRATES[@]} integration-test fixture crate(s) from tests/plugins/."
    fi
  fi

  # Build each product (and fixture) crate ONE AT A TIME, capturing
  # per-crate output to .logs/build/<crate>-<timestamp>.txt. cargo
  # shares the target dir across invocations so common deps compile
  # once; the per-crate split is purely for diagnosable, isolated logs
  # (the same shape as the test logs). Every crate is attempted so one
  # failure doesn't hide the rest.
  (
    cd "$REPO_ROOT"
    ws_failed=0
    # `${ARR[@]+"${ARR[@]}"}` expands to the elements when set and to
    # nothing when empty/unset — bash 3.2 (macOS) errors on a bare
    # `"${ARR[@]}"` of an empty array under `set -u`, which is exactly the
    # `--no-test-integration` case (FIXTURE_CRATES left empty).
    for crate in objectiveai-cli objectiveai-daemon objectiveai-api objectiveai-db objectiveai-laboratory ${FIXTURE_CRATES[@]+"${FIXTURE_CRATES[@]}"}; do
      if cargo build $PROFILE_FLAG -p "$crate" > "$LOG_DIR/${crate}-${BUILD_TS}.txt" 2>&1; then
        echo "$crate: SUCCESS"
      else
        echo "$crate: ERROR (see .logs/build/${crate}-${BUILD_TS}.txt)"
        ws_failed=1
      fi
    done
    [ "$ws_failed" -eq 0 ] || exit 1
  ) &
  CARGO_WORKSPACE_PID=$!

  # The viewer is a Tauri app: its build.sh runs `tauri build`, embedding the
  # frontend (vite, against the workspace @objectiveai/sdk dist) + the icon,
  # into objectiveai-viewer/embed/<profile>/. A plain `cargo build -p
  # objectiveai-viewer` would be a non-working dev-mode binary (no frontend,
  # no icon).
  #
  # Timing depends on whether the SDK is also being rebuilt this run:
  #   • --no-sdk (zip only): the committed sdk-js dist is final, so build the
  #     viewer now, concurrently with the cargo build + runners.
  #   • building both zip + SDK: DEFER the viewer until after phase 4's
  #     sdk-js build, so it embeds the freshly-built SDK rather than racing
  #     the dist that phase 4 is regenerating.
  if [ "$NO_SDK" = "1" ] && [ "$SKIP_VIEWER" != "1" ]; then
    bash "$REPO_ROOT/objectiveai-viewer/build.sh" $PROFILE_FLAG &
    VIEWER_PID=$!
  fi
fi

# ── Phases 2-4 (the SDK toolchain) ──────────────────────────────────────
# Skipped entirely under --no-sdk: the build/dev tools, the json
# schema, the wasm/cffi bindings, and the JS/Py/Go SDKs. Phase 1 +
# packaging don't depend on any of these.
if [ "$NO_SDK" != "1" ]; then
  # Phase 2 (parallel): build/dev tools + json schema.
  (
    if build_bin > "$LOG_DIR/build-bin.txt" 2>&1; then
      echo "build-bin: SUCCESS"
    else
      echo "build-bin: ERROR (see .logs/build/build-bin.txt)"
      exit 1
    fi
  ) &
  BUILD_BIN_PID=$!
  bash "$REPO_ROOT/objectiveai-json-schema/build.sh" &
  JSON_SCHEMA_PID=$!

  PHASE2_FAILED=false
  for pid in $BUILD_BIN_PID $JSON_SCHEMA_PID; do
    if ! wait "$pid"; then
      PHASE2_FAILED=true
    fi
  done
  if $PHASE2_FAILED; then
    exit 1
  fi

  # Phase 3: wasm + cffi (need the build tools from phase 2)
  run_phase objectiveai-sdk-rs-wasm-js/build.sh objectiveai-sdk-rs-cffi/build.sh

  # Phase 4: sdk-js FIRST (its freshly-rebuilt dist is what the viewer
  # embeds), then sdk-py + sdk-go in parallel — and, when also building the
  # zip, the deferred viewer build runs alongside them now that sdk-js is
  # done. objectiveai-dotnet is intentionally NOT part of this phase — its
  # codegen has a duplicate-variant-property bug that breaks on newly-added
  # internally-tagged enums; run `bash objectiveai-dotnet/build.sh` directly
  # if you need it. objectiveai-sdk-py compiles its own Rust extension
  # (_pyo3) via maturin as part of its build.
  run_phase objectiveai-sdk-js/build.sh

  # Deferred viewer (building both zip + SDK): the fresh sdk-js dist now
  # exists, so the viewer embeds the new SDK. Runs concurrently with py/go.
  if [ "$NO_ZIP" != "1" ] && [ "$SKIP_VIEWER" != "1" ]; then
    bash "$REPO_ROOT/objectiveai-viewer/build.sh" $PROFILE_FLAG &
    VIEWER_PID=$!
  fi

  run_phase objectiveai-sdk-py/build.sh objectiveai-sdk-go/build.sh
fi

# ── Integration-test importer prep (ordered AFTER the SDK build) ─────────
# The SDK importer test projects under tests/objectiveai-sdk-{go,py,js}-tests
# are *consumers* of the built SDK: the go importer's go.mod replaces the SDK
# module with the in-repo source, the js importer links @objectiveai/sdk's
# dist via the pnpm workspace, and the py importer runs under
# objectiveai-sdk-py's venv (built in phase 4). So they can only be prepared
# once the SDK build (phases 2-4) is done — hence this runs after that block.
# Under --no-sdk those phases are skipped and this prep runs against whatever
# SDK artifacts already exist (the "use the existing as-is" contract).
# Skipped by --no-test-integration (the release passes it, since it ships
# zips, not tests).
if [ "$NO_TEST_INTEGRATION" != "1" ]; then
  if (
    set -e
    # JS importer: pnpm workspace link to the freshly-built @objectiveai/sdk
    # dist (pnpm install is idempotent and picks up the new workspace package
    # the first time it runs). --no-frozen-lockfile so adding the importer as
    # a workspace member can update pnpm-lock.yaml even when pnpm would
    # otherwise auto-freeze (e.g. under CI).
    pnpm install --no-frozen-lockfile
    # Go importer: materialize its module graph. The transitive deps are
    # already in the module cache from the SDK build; the replace directive
    # points at the in-repo SDK source.
    ( cd "$REPO_ROOT/tests/objectiveai-sdk-go-tests" && go mod download )
    # Py importer: nothing to prep — it runs under objectiveai-sdk-py/venv,
    # which phase 4 already built with the SDK installed.
  ) > "$LOG_DIR/integration-prep.txt" 2>&1; then
    echo "integration-prep: SUCCESS"
  else
    echo "integration-prep: ERROR (see .logs/build/integration-prep.txt)"
    exit 1
  fi
fi

# Wait for the background phase-1 jobs (cargo bins + viewer + runners).
# Skipped under --no-zip (phase 1 never launched).
if [ "$NO_ZIP" != "1" ]; then
  FAILED=false
  for pid in $CLAUDE_RUNNER_PID $CODEX_RUNNER_PID $CARGO_WORKSPACE_PID $VIEWER_PID $LAB_PID $DB_PROXY_PID; do
    if ! wait "$pid"; then
      FAILED=true
    fi
  done

  if $FAILED; then
    exit 1
  fi
fi

# ── Package the host's 9 binaries into <dir>/bin/<release-asset>.zip ─────
# Bundles the freshly-built binaries into the same per-platform zip the
# GitHub Release ships (objectiveai-<version>-<os>-<arch>.zip) and drops
# it in <OBJECTIVEAI_DIR>/bin so the installer / `objectiveai update` can
# pick it up locally. Host platform only — not the other 5. Uses `python
# -m zipfile` (cross-platform; `zip(1)` is absent in Git Bash). The thin
# `objectiveai-cli` crate builds the `objectiveai` binary; the resident
# `objectiveai-daemon` crate builds its own like-named binary. The
# version is read from objectiveai-daemon/Cargo.toml — the canonical release
# version (release.yml gates on it, version.sh keeps install.sh in sync).
package_host_zip() {
  local os arch ext profile host_triple
  case "$(uname -s)" in
    Linux*)               os="linux"   ;;
    Darwin*)              os="macos"   ;;
    CYGWIN*|MINGW*|MSYS*) os="windows" ;;
    *) echo "package: unsupported OS: $(uname -s)" >&2; return 1 ;;
  esac
  arch=$(uname -m)
  case "$arch" in
    x86_64|amd64)  arch="x86_64"  ;;
    arm64|aarch64) arch="aarch64" ;;
    *) echo "package: unsupported architecture: $arch" >&2; return 1 ;;
  esac
  [ "$os" = "windows" ] && ext=".exe" || ext=""
  profile="debug"; [ "$RELEASE" = "1" ] && profile="release"
  host_triple=$(rustc -vV | awk '/^host:/{print $2}')

  local py
  py=$(command -v python3 || command -v python || true)
  [ -n "$py" ] || { echo "package: need python3 to build the zip" >&2; return 1; }

  local version
  version=$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$REPO_ROOT/objectiveai-daemon/Cargo.toml" | head -1)
  [ -n "$version" ] || { echo "package: could not read version from objectiveai-daemon/Cargo.toml" >&2; return 1; }

  local asset="objectiveai-${version}-${os}-${arch}.zip"
  local install_dir="${OBJECTIVEAI_DIR:-$HOME/.objectiveai}"
  local bin_dir="$install_dir/bin"
  mkdir -p "$bin_dir"

  # Stage the 10 binaries under their shipped names (built-name -> ship-name).
  local stage="$REPO_ROOT/target/.package-stage.$$"
  rm -rf "$stage"; mkdir -p "$stage"

  local cargo_dir="$REPO_ROOT/target/$profile"
  local src
  # The CLI + daemon + server crates from the cargo build (built-name ->
  # ship-name; the thin objectiveai-cli crate builds the `objectiveai`
  # binary, and the daemon crate builds/ships `objectiveai-daemon`).
  local pairs="objectiveai|objectiveai objectiveai-daemon|objectiveai-daemon objectiveai-api|objectiveai-api objectiveai-db|objectiveai-db objectiveai-laboratory|objectiveai-laboratory"
  local entry built ship
  for entry in $pairs; do
    built="${entry%%|*}"; ship="${entry##*|}"
    src="$cargo_dir/$built$ext"
    if [ ! -f "$src" ]; then
      echo "package: missing $src" >&2; rm -rf "$stage"; return 1
    fi
    cp "$src" "$stage/$ship$ext"
  done

  # The viewer comes from its `tauri build` (objectiveai-viewer/build.sh),
  # which places it in objectiveai-viewer/embed/<profile>/ — NOT target/.
  # Under --skip-viewer the zip simply ships without it (an install keeps
  # whatever viewer binary it already has).
  if [ "$SKIP_VIEWER" != "1" ]; then
    src="$REPO_ROOT/objectiveai-viewer/embed/$profile/objectiveai-viewer$ext"
    if [ ! -f "$src" ]; then
      echo "package: missing $src (run objectiveai-viewer/build.sh)" >&2; rm -rf "$stage"; return 1
    fi
    cp "$src" "$stage/objectiveai-viewer$ext"
  fi

  local r
  for r in objectiveai-claude-agent-sdk-runner objectiveai-codex-sdk-runner; do
    src="$REPO_ROOT/$r/embed/$host_triple/$profile/$r$ext"
    if [ ! -f "$src" ]; then
      echo "package: missing $src" >&2; rm -rf "$stage"; return 1
    fi
    cp "$src" "$stage/$r$ext"
  done

  # objectiveai-mcp-laboratory — always a musl-linux binary (no ext, fixed
  # name), arch-matched to the host; it runs inside Docker lab containers, not
  # on the host. Phase 1 guarantees it (built from source, or pre-placed in
  # CI), so a missing binary here is a hard error like the other binaries.
  local lab_triple="${arch}-unknown-linux-musl"
  src="$REPO_ROOT/objectiveai-mcp-laboratory/embed/$lab_triple/$profile/objectiveai-mcp-laboratory"
  if [ ! -f "$src" ]; then
    echo "package: missing $src" >&2; rm -rf "$stage"; return 1
  fi
  cp "$src" "$stage/objectiveai-mcp-laboratory"

  # objectiveai-db-proxy — likewise a musl-linux binary that runs inside
  # containers rather than on the host, arch-matched and guaranteed by
  # phase 1, so a missing one is a hard error too.
  src="$REPO_ROOT/objectiveai-db-proxy/embed/$lab_triple/$profile/objectiveai-db-proxy"
  if [ ! -f "$src" ]; then
    echo "package: missing $src" >&2; rm -rf "$stage"; return 1
  fi
  cp "$src" "$stage/objectiveai-db-proxy"

  # Build the zip to a temp path, then move into place (no partial asset).
  local out="$bin_dir/$asset" tmp="$bin_dir/$asset.partial.$$"
  rm -f "$tmp"
  if ! ( cd "$stage" && "$py" -m zipfile -c "$tmp" * ); then
    rm -f "$tmp"; rm -rf "$stage"
    echo "package: zip creation failed" >&2; return 1
  fi
  mv -f "$tmp" "$out"
  rm -rf "$stage"
  echo "Packaged $out ($profile)"
}

if [ "$NO_ZIP" != "1" ]; then
  if ! package_host_zip; then
    exit 1
  fi
fi
