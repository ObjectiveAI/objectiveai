#!/usr/bin/env bash
# test.sh — top-level test launcher.
#
# Runs the enabled suites in parallel and reports their aggregate result:
#   - unit         (test-unit.sh)        — Rust unit/in-crate tests; no server
#   - sdk          (test-sdk.sh)         — go/py/js SDK unit tests; no server
#   - integration  (test-integration.sh) — Rust integration crates + the SDK
#                                           importer projects; this suite owns
#                                           the shared API server lifecycle.
#
# The .objectiveai reset, binary (re)install, server spawn, and teardown that
# used to live here moved into test-integration.sh — only the integration
# suite needs a server now, so unit/sdk run with no setup at all.
#
# Flags (all optional): --no-unit, --no-sdk, --no-integration.
#
# Usage:
#   bash test.sh [--no-unit] [--no-sdk] [--no-integration]

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
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

# Launch the enabled suites in parallel. Each suite self-provisions whatever
# it needs (only integration needs a server, and it owns that lifecycle).
pids=()
names=()
launch() { bash "$REPO_ROOT/$2" & pids+=("$!"); names+=("$1"); }
[ "$NO_UNIT" = "1" ]        || launch unit        test-unit.sh
[ "$NO_SDK" = "1" ]         || launch sdk         test-sdk.sh
[ "$NO_INTEGRATION" = "1" ] || launch integration test-integration.sh

if [ "${#pids[@]}" -eq 0 ]; then
  echo "test: nothing to run (--no-unit --no-sdk --no-integration)"
  exit 0
fi

failed=0
for i in "${!pids[@]}"; do
  if wait "${pids[$i]}"; then
    echo "test: ${names[$i]} suite: PASS"
  else
    echo "test: ${names[$i]} suite: FAIL"
    failed=1
  fi
done

exit "$failed"
