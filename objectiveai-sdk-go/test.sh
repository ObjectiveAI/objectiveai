#!/usr/bin/env bash
# Run the Go SDK tests against an already-running API server.
#
# OBJECTIVEAI_ADDRESS must point at that server's base URL — this script
# does NOT spawn or discover one, and exits non-zero if it is unset.
# Output goes straight to stdout/stderr; the caller captures it.
#
# Usage:
#   OBJECTIVEAI_ADDRESS=http://127.0.0.1:8080 bash objectiveai-sdk-go/test.sh
#   OBJECTIVEAI_ADDRESS=http://127.0.0.1:8080 bash objectiveai-sdk-go/test.sh -run TestRoundtrip

set -euo pipefail

: "${OBJECTIVEAI_ADDRESS:?must be set to the running API server base URL}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
go test ./tests/ ./ -v -count=1 "$@"
