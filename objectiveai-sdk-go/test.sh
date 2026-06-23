#!/usr/bin/env bash
# Run the Go SDK unit tests (offline — no server required).
#
# The HTTP/snapshot tests that needed a running API server were moved to the
# standalone importer project tests/objectiveai-sdk-go-tests/. What remains
# here (merge/push, schema roundtrip, cffi/http coverage) hits no network.
# Output goes straight to stdout/stderr; the caller captures it.
#
# Usage:
#   bash objectiveai-sdk-go/test.sh
#   bash objectiveai-sdk-go/test.sh -run TestRoundtrip

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
go test ./tests/ ./ -v -count=1 "$@"
