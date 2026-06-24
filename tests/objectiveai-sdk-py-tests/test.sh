#!/usr/bin/env bash
# Run the Python SDK HTTP/snapshot integration tests — a standalone project
# that imports the built Python SDK and drives a running API server.
#
# Executes via objectiveai-sdk-py's venv, which holds the freshly-built
# `objectiveai_sdk` package (maturin develop) plus pytest/pytest-asyncio —
# so this is a true importer of the built artifact with no second build.
# OBJECTIVEAI_ADDRESS should point at a running server; tests skip when it
# is unset. Output goes straight to stdout/stderr; the caller captures it.
#
# Usage:
#   OBJECTIVEAI_ADDRESS=http://127.0.0.1:8080 bash tests/objectiveai-sdk-py-tests/test.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VENV_DIR="$SCRIPT_DIR/../../objectiveai-sdk-py/venv"
if [ -d "$VENV_DIR/Scripts" ]; then
  PYTHON="$VENV_DIR/Scripts/python.exe"
else
  PYTHON="$VENV_DIR/bin/python"
fi

cd "$SCRIPT_DIR"
"$PYTHON" -m pytest tests/ -v --tb=long "$@"
