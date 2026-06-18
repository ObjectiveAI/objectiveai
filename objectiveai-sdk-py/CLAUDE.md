# objectiveai-sdk-py

Python SDK for ObjectiveAI. Published to PyPI as `objectiveai-sdk`.

## Layout (maturin mixed Python/Rust)

The Rust crate lives in the sibling `../objectiveai-sdk-rs-pyo3/` directory and
is referenced from `pyproject.toml` via `[tool.maturin] manifest-path`.

```
objectiveai-sdk-py/
├── pyproject.toml          # build-backend = "maturin", manifest-path → ../objectiveai-sdk-rs-pyo3/Cargo.toml
├── objectiveai_sdk/        # pure-Python sources
│   ├── __init__.py
│   ├── client.py
│   ├── ...
│   └── _pyo3.<abi>.pyd     # built by maturin into the package at install/develop time
├── scripts/install_pydantic.py
├── tests/
├── build.sh, test.sh
└── requirements.txt, requirements-dev.txt

../objectiveai-sdk-rs-pyo3/
├── Cargo.toml              # workspace member, [lib] crate-type = ["cdylib"], name = "_pyo3"
└── src/lib.rs              # #[pymodule] fn _pyo3(...) with all #[pyfunction] entries
```

The Rust extension is bundled into the same wheel as the pure-Python sources — no separate `objectiveai_sdk_pyo3` PyPI package. `maturin develop` builds the extension and editable-installs the package into the venv.

## Virtual Environment

**CRITICAL: Never run bare `python` or `pip` commands.** Always use the venv:

```bash
# Windows
objectiveai-sdk-py/venv/Scripts/python.exe <args>
objectiveai-sdk-py/venv/Scripts/pip.exe <args>

# Running scripts
objectiveai-sdk-py/venv/Scripts/python.exe objectiveai-sdk-py/scripts/install_pydantic.py

# Running tests
objectiveai-sdk-py/venv/Scripts/python.exe -m pytest objectiveai-sdk-py/tests/ <args>
```

## Build

```bash
bash objectiveai-sdk-py/build.sh
```

The flow: venv setup → install requirements → `install_pydantic.py` (codegen → `objectiveai_sdk/`) → `maturin develop --release` (compiles Rust extension and installs editable).

## Code Generation

Pydantic types under `objectiveai_sdk/` are auto-generated from `../objectiveai-json-schema/`. Do not edit files containing the `THIS FILE IS AUTO-GENERATED` header.

```bash
objectiveai-sdk-py/venv/Scripts/python.exe objectiveai-sdk-py/scripts/install_pydantic.py
```

## Imports

The compiled extension lives at `objectiveai_sdk._pyo3`. Internal `pyo3.py` thin wrappers alias it:

```python
import objectiveai_sdk._pyo3 as objectiveai_sdk_pyo3
```

Then call `objectiveai_sdk_pyo3.<func>(...)` as before.

## Tests

```bash
# All tests
objectiveai-sdk-py/venv/Scripts/python.exe -m pytest objectiveai-sdk-py/tests/ -x --tb=short

# Roundtrip test only
objectiveai-sdk-py/venv/Scripts/python.exe -m pytest objectiveai-sdk-py/tests/test_pydantic_roundtrip.py -x --tb=short
```

## Publish

There is no per-package publish script. The unified `Release` GitHub Actions
workflow (`.github/workflows/release.yml`) publishes everything on a
CLI-version bump: its `python` jobs build cross-platform wheels
(linux-x86_64, linux-aarch64, macos-arm64, windows-x86_64) plus an sdist via
maturin — using the committed Pydantic types (no codegen) — and upload to
PyPI with the `PYPI_API_TOKEN` repo secret (`skip-existing`).

The wheels use `pyo3 = { features = ["abi3-py310"] }`, so each per-platform
wheel is forward-compatible across CPython 3.10+ — only one wheel per
platform, not one per (platform × Python version).
