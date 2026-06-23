#!/usr/bin/env bash
# Run the Go SDK HTTP/snapshot integration tests — a standalone module that
# imports the built Go SDK (via the replace directive in go.mod) and drives a
# running API server.
#
# OBJECTIVEAI_ADDRESS should point at that server's base URL; individual
# tests skip themselves when it is unset. Output goes straight to
# stdout/stderr; the caller captures it.
#
# Usage:
#   OBJECTIVEAI_ADDRESS=http://127.0.0.1:8080 bash tests/objectiveai-sdk-go-tests/test.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"
go test ./... -v -count=1 "$@"
