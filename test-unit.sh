#!/usr/bin/env bash
# test-unit.sh — run the unit/in-crate test suites in parallel.
#
# For every workspace crate whose manifest does NOT live under tests/
# (i.e. the product + SDK crates, excluding the integration fixtures and
# invariant-test crates parked in tests/), run `cargo nextest` against it
# and capture its output to .logs/tests/<crate>-tests-<timestamp>.txt.
# All crates run concurrently; the script waits for every one.
#
# Uses the HOST cargo-nextest — whatever `cargo nextest` resolves to on
# PATH — NOT the repo's pinned bin/cargo-nextest.
#
# Exit status: 0 iff every per-crate run exited 0; 1 if any run failed.
#
# Usage:
#   bash test-unit.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$REPO_ROOT/.logs/tests"
mkdir -p "$LOG_DIR"

# Host nextest, explicitly NOT $REPO_ROOT/bin/cargo-nextest.
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
tests_prefix = root + "/tests/"
members = set(md["workspace_members"])
names = set()
for p in md["packages"]:
    if p["id"] not in members:
        continue
    manifest = p["manifest_path"].replace("\\", "/")
    if manifest.startswith(tests_prefix):
        continue
    names.add(p["name"])
for n in sorted(names):
    print(n)
'
)

if [ "${#CRATES[@]}" -eq 0 ]; then
  echo "test-unit: no crates to test" >&2
  exit 1
fi

echo "test-unit: running ${#CRATES[@]} crate(s) via host cargo-nextest -> $LOG_DIR"

# Launch one nextest run per crate, all in parallel.
pids=()
pid_crates=()
for crate in "${CRATES[@]}"; do
  log="$LOG_DIR/${crate}-tests-${TIMESTAMP}.txt"
  cargo nextest run --manifest-path "$REPO_ROOT/Cargo.toml" -p "$crate" >"$log" 2>&1 &
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
