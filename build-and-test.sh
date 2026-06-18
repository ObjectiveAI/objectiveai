#!/usr/bin/env bash
# build-and-test.sh — development helper: build everything the test run
# needs, then run it.
#
# Runs two build.sh invocations in parallel:
#   --no-zip --no-test-integration --release  → the SDK toolchain (release)
#   --no-sdk                                   → product binaries + zip + fixtures
# Waits for both and exits 1 if either failed. Then runs test.sh and
# exits with its exit code.
#
# Usage:
#   bash build-and-test.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

bash "$REPO_ROOT/build.sh" --no-zip --no-test-integration --release & B1=$!
bash "$REPO_ROOT/build.sh" --no-sdk & B2=$!

failed=0
wait "$B1" || failed=1
wait "$B2" || failed=1
if [ "$failed" -ne 0 ]; then
  echo "build-and-test: build failed" >&2
  exit 1
fi

exec bash "$REPO_ROOT/test.sh"
