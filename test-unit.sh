#!/usr/bin/env bash
# test-unit.sh — run the unit/in-crate test suites in parallel.
#
# For every workspace crate EXCEPT the two integration-test crates
# (objectiveai-api-tests, objectiveai-cli-tests under tests/), run
# `cargo nextest` against it and capture its output to
# .logs/tests/<crate>-tests-<timestamp>.txt. All crates run
# concurrently; the script waits for every one. test-integration.sh
# runs the two excluded crates.
#
# Uses the host cargo-nextest — whatever `cargo nextest` resolves to on
# PATH.
#
# Exit status: 0 iff every per-crate run exited 0; 1 if any run failed.
#
# Usage:
#   bash test-unit.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$REPO_ROOT/.logs/tests"
mkdir -p "$LOG_DIR"

# Host nextest (whatever `cargo nextest` resolves to on PATH).
if ! cargo nextest --version >/dev/null 2>&1; then
  echo "test-unit: host cargo-nextest not found on PATH; install it (cargo install cargo-nextest)" >&2
  exit 1
fi

PY="$(command -v python3 || command -v python || true)"
[ -n "$PY" ] || { echo "test-unit: need python to enumerate workspace crates" >&2; exit 1; }

# One timestamp for the whole run, so a run's logs sort together.
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

# Workspace member crates whose manifest is NOT under <repo>/tests/.
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
    if any(manifest.startswith(d) for d in integ):
        continue
    names.add(p["name"])
for n in sorted(names):
    print(n)
' | tr -d '\r'
)

if [ "${#CRATES[@]}" -eq 0 ]; then
  echo "test-unit: no crates to test" >&2
  exit 1
fi

echo "test-unit: running ${#CRATES[@]} crate(s) via host cargo-nextest -> $LOG_DIR"

# Build each crate's test binaries up front, ONE AT A TIME, capturing
# per-crate output to .logs/build/<crate>-nextest-<timestamp>.txt.
# Sequential (not the parallel run pattern below) so concurrent full
# builds don't oversubscribe the shared target dir; with the binaries
# prebuilt, the parallel run phase only executes tests. Every crate is
# attempted so one broken build doesn't hide the others.
BUILD_LOG_DIR="$REPO_ROOT/.logs/build"
mkdir -p "$BUILD_LOG_DIR"
prebuild_failed=0
for crate in "${CRATES[@]}"; do
  echo "test-unit: build $crate ..."
  if ! cargo nextest run --no-run --manifest-path "$REPO_ROOT/Cargo.toml" -p "$crate" \
       >"$BUILD_LOG_DIR/${crate}-nextest-${TIMESTAMP}.txt" 2>&1; then
    echo "test-unit: BUILD FAILED: $crate (see .logs/build/${crate}-nextest-${TIMESTAMP}.txt)" >&2
    prebuild_failed=1
  fi
done
if [ "$prebuild_failed" -ne 0 ]; then
  echo "test-unit: one or more test builds failed; aborting" >&2
  exit 1
fi

# Launch one nextest run per crate, all in parallel.
pids=()
pid_crates=()
for crate in "${CRATES[@]}"; do
  log="$LOG_DIR/${crate}-tests-${TIMESTAMP}.txt"
  cargo nextest run --no-tests=pass --manifest-path "$REPO_ROOT/Cargo.toml" -p "$crate" >"$log" 2>&1 &
  pids+=("$!")
  pid_crates+=("$crate")
done

# Wait for all; aggregate exit codes (any failure -> overall failure).
failed=0
for i in "${!pids[@]}"; do
  if wait "${pids[$i]}"; then
    echo "test-unit: ${pid_crates[$i]}: PASS"
  else
    echo "test-unit: ${pid_crates[$i]}: FAIL"
    failed=1
  fi
done

exit "$failed"
