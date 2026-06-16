#!/usr/bin/env bash
# Build-only: compile this suite's test binaries without running them,
# so the root `test.sh` can pre-build them up front (before any suite
# spawns a server) and the parallel test phase never relinks a running
# `.exe` (the relink race). `nextest list` builds exactly what this
# suite's `test.sh` `nextest run` builds, so the later run is a cache
# hit. Mirror that invocation so the fingerprints match.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

# Build the committed .objectiveai fixture crates (plugins + tools) in
# debug, up front, to target/debug. Their manifests' `exec` points at the
# prebuilt binary (a relative path into target/debug) instead of
# `cargo run`, so the parallel test phase never relinks a running fixture
# `.exe` — the same relink race we already avoid for the test binaries.
#
# The set is discovered from the workspace members: every `.objectiveai/…`
# entry in the root Cargo.toml, with each crate's package name read from
# its own Cargo.toml. A new fixture crate is picked up automatically — no
# edit here when one is added.
fixture_pkgs=()
while IFS= read -r member; do
  cargo_toml="$REPO_ROOT/$member/Cargo.toml"
  [ -f "$cargo_toml" ] || continue
  name=$(grep -m1 -E '^name[[:space:]]*=[[:space:]]*"' "$cargo_toml" \
    | sed -E 's/^name[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/')
  [ -n "$name" ] && fixture_pkgs+=(-p "$name")
done < <(grep -oE '"\.objectiveai/[^"]*"' "$REPO_ROOT/Cargo.toml" | tr -d '"')

if [ "${#fixture_pkgs[@]}" -gt 0 ]; then
  cargo build --manifest-path "$REPO_ROOT/Cargo.toml" "${fixture_pkgs[@]}"
fi

"$NEXTEST" nextest list --manifest-path "$SCRIPT_DIR/Cargo.toml" >/dev/null
