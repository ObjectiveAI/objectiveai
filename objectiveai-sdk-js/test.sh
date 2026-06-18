#!/usr/bin/env bash
# Run the JavaScript SDK tests against an already-running API server.
#
# OBJECTIVEAI_TEST_PORT must point at that server's port — this script
# does NOT spawn or discover one, and exits non-zero if it is unset.
# Output goes straight to stdout/stderr; the caller captures it.
#
# Usage:
#   OBJECTIVEAI_TEST_PORT=8080 bash objectiveai-sdk-js/test.sh

set -euo pipefail

: "${OBJECTIVEAI_TEST_PORT:?must be set to the running API server port}"

pnpm --filter @objectiveai/sdk run test -- --reporter=verbose "$@"
