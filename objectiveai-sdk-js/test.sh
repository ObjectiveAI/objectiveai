#!/usr/bin/env bash
# Run the JS SDK unit tests (offline — no server required).
#
# The HTTP/snapshot tests that needed a running API server were moved to the
# standalone importer project tests/objectiveai-sdk-js-tests/. What remains
# here (merge, zod roundtrip, wasm/export coverage, built-artifact smoke
# tests, typecheck) hits no network.
#
# Usage:
#   bash objectiveai-sdk-js/test.sh

set -euo pipefail

pnpm --filter @objectiveai/sdk run test -- --reporter=verbose "$@"
