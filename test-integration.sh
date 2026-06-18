#!/usr/bin/env bash
# test-integration.sh — run the integration-test crates in parallel.
#
# Runs `cargo nextest` against the two integration-test crates parked
# under tests/ (objectiveai-api-tests, objectiveai-cli-tests), capturing
# each to .logs/tests/<crate>-<timestamp>.txt. All run concurrently; the
# script waits for every one. test-unit.sh runs everything else.
#
# Uses the HOST cargo-nextest — whatever `cargo nextest` resolves to on
# PATH — NOT the repo's pinned bin/cargo-nextest.
#
# Exit status: 0 iff every run exited 0; 1 if any failed.
#
# Usage:
#   bash test-integration.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$REPO_ROOT/.logs/tests"
mkdir -p "$LOG_DIR"

# Host nextest, explicitly NOT $REPO_ROOT/bin/cargo-nextest.
if ! cargo nextest --version >/dev/null 2>&1; then
  echo "test-integration: host cargo-nextest not found on PATH; install it (cargo install cargo-nextest)" >&2
  exit 1
fi

PY="$(command -v python3 || command -v python || true)"
[ -n "$PY" ] || { echo "test-integration: need python to enumerate workspace crates" >&2; exit 1; }

# One timestamp for the whole run, so a run's logs sort together.
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

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
'
)

if [ "${#CRATES[@]}" -eq 0 ]; then
  echo "test-integration: no integration crates found (nothing to run)"
  exit 0
fi

echo "test-integration: running ${#CRATES[@]} crate(s) via host cargo-nextest -> $LOG_DIR"

# Launch one nextest run per crate, all in parallel.
pids=()
pid_crates=()
for crate in "${CRATES[@]}"; do
  log="$LOG_DIR/${crate}-${TIMESTAMP}.txt"
  cargo nextest run --manifest-path "$REPO_ROOT/Cargo.toml" -p "$crate" >"$log" 2>&1 &
  pids+=("$!")
  pid_crates+=("$crate")
done

# Wait for all; aggregate exit codes (any failure -> overall failure).
failed=0
for i in "${!pids[@]}"; do
  if wait "${pids[$i]}"; then
    echo "test-integration: ${pid_crates[$i]}: PASS"
  else
    echo "test-integration: ${pid_crates[$i]}: FAIL"
    failed=1
  fi
done

exit "$failed"
