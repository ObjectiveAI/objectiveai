#!/usr/bin/env bash
# build-and-test.sh — development helper: build everything the test run
# needs, then run it.
#
# Runs two build.sh invocations, then the test suite:
#   1. --no-zip --no-test-integration --release  → the SDK toolchain (release)
#   2. --no-sdk                                   → product binaries + zip + fixtures
# If either build fails, exit 1. Otherwise run test.sh and exit with its code.
#
# These run SEQUENTIALLY, not in parallel: both touch the shared pnpm /
# JS workspace. The SDK build rebuilds objectiveai-sdk-js (its build
# script `rm -rf dist` then regenerates it) and runs pnpm, while the
# product build's viewer step consumes @objectiveai/sdk (tsc resolves it
# against that dist) and runs `pnpm install`. Running them concurrently
# races — the viewer sees a half-wiped sdk-js dist (TS2307) or a
# node_modules mid-churn (missing vite / broken workspace links). Build
# the SDK first so the product build sees a stable, complete sdk-js.
#
# Usage:
#   bash build-and-test.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

if ! bash "$REPO_ROOT/build.sh" --no-zip --no-test-integration --release; then
  echo "build-and-test: SDK toolchain build failed" >&2
  exit 1
fi
if ! bash "$REPO_ROOT/build.sh" --no-sdk; then
  echo "build-and-test: product build failed" >&2
  exit 1
fi

exec bash "$REPO_ROOT/test.sh"
