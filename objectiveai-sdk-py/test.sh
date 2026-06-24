#!/usr/bin/env bash
# Run the Python SDK unit tests (offline — no server required).
#
# The HTTP/snapshot tests that needed a running API server were moved to the
# standalone importer project tests/objectiveai-sdk-py-tests/. What remains
# here (merge/push, pydantic roundtrip, pyo3/http coverage) hits no network.
# Output goes straight to stdout/stderr; the caller captures it.
#
# Usage:
#   bash objectiveai-sdk-py/test.sh
#   bash objectiveai-sdk-py/test.sh -k mock_7 -vv

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
if [ -d "$VENV_DIR/Scripts" ]; then
  PYTHON="$VENV_DIR/Scripts/python.exe"
else
  PYTHON="$VENV_DIR/bin/python"
fi

"$PYTHON" -m pytest "$SCRIPT_DIR/tests/" -v --tb=long "$@"
