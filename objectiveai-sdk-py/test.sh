#!/usr/bin/env bash
# Run the Python SDK tests against an already-running API server.
#
# OBJECTIVEAI_TEST_PORT must point at that server's port — this script
# does NOT spawn or discover one, and exits non-zero if it is unset.
# Output goes straight to stdout/stderr; the caller captures it.
#
# Usage:
#   OBJECTIVEAI_TEST_PORT=8080 bash objectiveai-sdk-py/test.sh
#   OBJECTIVEAI_TEST_PORT=8080 bash objectiveai-sdk-py/test.sh -k mock_7 -vv

set -euo pipefail

: "${OBJECTIVEAI_TEST_PORT:?must be set to the running API server port}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
if [ -d "$VENV_DIR/Scripts" ]; then
  PYTHON="$VENV_DIR/Scripts/python.exe"
else
  PYTHON="$VENV_DIR/bin/python"
fi

"$PYTHON" -m pytest "$SCRIPT_DIR/tests/" -v --tb=long "$@"
