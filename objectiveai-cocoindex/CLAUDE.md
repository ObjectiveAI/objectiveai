# objectiveai-cocoindex

ObjectiveAI integration for [cocoindex](https://github.com/cocoindex-io/cocoindex).

## Dependencies

Canonical declaration is in `pyproject.toml` `[project.dependencies]`:
```
objectiveai-sdk==X.Y.Z
cocoindex
```
This is what downstream `pip install objectiveai-cocoindex` users actually
get. `requirements.txt` mirrors the same pin (kept for callers that
`pip install -r requirements.txt` directly). Both are bumped in lockstep
by `bash version.sh <new-version>`.

For local dev, `build.sh`:
1. editable-installs `../objectiveai-sdk-py` first, so `objectiveai-sdk` lands in
   the venv at the same `X.Y.Z` the pin requires (live-edits picked up);
2. editable-installs this package (`pip install -e .`), which pulls
   `cocoindex` from PyPI and confirms `objectiveai-sdk==X.Y.Z` is satisfied
   by the editable sibling install.

When users `pip install objectiveai-cocoindex` from PyPI, this redirect
doesn't fire — pip pulls `objectiveai-sdk` and `cocoindex` from PyPI normally.

Maturin compiles `objectiveai_sdk._pyo3` into the venv on the sibling install,
so a Rust toolchain must be available for local dev.

## Virtual Environment

**CRITICAL: Never run bare `python` or `pip` commands.** Always use the venv:

```bash
# Windows
objectiveai-cocoindex/venv/Scripts/python.exe <args>
objectiveai-cocoindex/venv/Scripts/pip.exe <args>

# Linux/macOS
objectiveai-cocoindex/venv/bin/python <args>
objectiveai-cocoindex/venv/bin/pip <args>
```

## Build

```bash
bash objectiveai-cocoindex/build.sh
```

Creates the venv (if missing) and installs `requirements.txt` + `requirements-dev.txt`.

## Tests

```bash
bash objectiveai-cocoindex/test.sh
bash objectiveai-cocoindex/test.sh -- -k foo -vv
```
