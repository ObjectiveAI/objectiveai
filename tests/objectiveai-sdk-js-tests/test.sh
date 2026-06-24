#!/usr/bin/env bash
# Run the JS SDK HTTP/snapshot integration tests — a standalone workspace
# package that imports the built @objectiveai/sdk (resolved to dist via the
# pnpm workspace link) and drives a running API server.
#
# OBJECTIVEAI_ADDRESS should point at that server's base URL; individual
# suites skip when it is unset. Output goes straight to stdout/stderr; the
# caller captures it.
#
# Usage:
#   OBJECTIVEAI_ADDRESS=http://127.0.0.1:8080 bash tests/objectiveai-sdk-js-tests/test.sh

set -euo pipefail

pnpm --filter @objectiveai/sdk-tests run test -- --reporter=verbose "$@"
