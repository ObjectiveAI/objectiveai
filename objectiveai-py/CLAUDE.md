# objectiveai-py

Python SDK for ObjectiveAI. Published to PyPI as `objectiveai`.

## Layout (maturin mixed Python/Rust)

The Rust crate lives in the sibling `../objectiveai-rs-pyo3/` directory and
is referenced from `pyproject.toml` via `[tool.maturin] manifest-path`.

```
objectiveai-py/
├── pyproject.toml          # build-backend = "maturin", manifest-path → ../objectiveai-rs-pyo3/Cargo.toml
├── objectiveai/            # pure-Python sources
│   ├── __init__.py
│   ├── client.py
│   ├── ...
│   └── _pyo3.<abi>.pyd     # built by maturin into the package at install/develop time
├── scripts/install_pydantic.py
├── tests/
├── build.sh, test.sh, publish.sh
└── requirements.txt, requirements-dev.txt

../objectiveai-rs-pyo3/
├── Cargo.toml              # workspace member, [lib] crate-type = ["cdylib"], name = "_pyo3"
└── src/lib.rs              # #[pymodule] fn _pyo3(...) with all #[pyfunction] entries
```

The Rust extension is bundled into the same wheel as the pure-Python sources — no separate `objectiveai_pyo3` PyPI package. `maturin develop` builds the extension and editable-installs the package into the venv.

## Virtual Environment

**CRITICAL: Never run bare `python` or `pip` commands.** Always use the venv:

```bash
# Windows
objectiveai-py/venv/Scripts/python.exe <args>
objectiveai-py/venv/Scripts/pip.exe <args>

# Running scripts
objectiveai-py/venv/Scripts/python.exe objectiveai-py/scripts/install_pydantic.py

# Running tests
objectiveai-py/venv/Scripts/python.exe -m pytest objectiveai-py/tests/ <args>
```

## Build

```bash
bash objectiveai-py/build.sh
```

The flow: venv setup → install requirements → `install_pydantic.py` (codegen → `objectiveai/`) → `maturin develop --release` (compiles Rust extension and installs editable).

## Code Generation

Pydantic types under `objectiveai/` are auto-generated from `../objectiveai-json-schema/`. Do not edit files containing the `THIS FILE IS AUTO-GENERATED` header.

```bash
objectiveai-py/venv/Scripts/python.exe objectiveai-py/scripts/install_pydantic.py
```

## Imports

The compiled extension lives at `objectiveai._pyo3`. Internal `pyo3.py` thin wrappers alias it:

```python
import objectiveai._pyo3 as objectiveai_pyo3
```

Then call `objectiveai_pyo3.<func>(...)` as before.

## Tests

```bash
# All tests
objectiveai-py/venv/Scripts/python.exe -m pytest objectiveai-py/tests/ -x --tb=short

# Roundtrip test only
objectiveai-py/venv/Scripts/python.exe -m pytest objectiveai-py/tests/test_pydantic_roundtrip.py -x --tb=short
```

## Publish

Real publishes are cross-platform via the GitHub Actions workflow at
`.github/workflows/publish-objectiveai-py.yml`. It builds wheels for
linux-x86_64, linux-aarch64, macos-x86_64, macos-arm64, windows-x86_64
plus an sdist, then uploads to PyPI via Trusted Publishing.

```bash
bash objectiveai-py/publish.sh                  # PyPI (cross-platform via GHA)
bash objectiveai-py/publish.sh --test           # TestPyPI (cross-platform via GHA)
bash objectiveai-py/publish.sh --build-only     # local single-platform sanity check
```

The wheels use `pyo3 = { features = ["abi3-py310"] }`, so each per-platform
wheel is forward-compatible across CPython 3.10+ — only one wheel per
platform, not one per (platform × Python version).

### One-time setup for Trusted Publishing

1. Configure trusted publishing on PyPI:
   https://pypi.org/manage/project/objectiveai/settings/publishing/
   Add a "GitHub Actions" trusted publisher pointing at this workflow with
   environment name `pypi` (and `testpypi` if you also want test uploads).
2. Create matching environments in this repo's settings → environments.
3. `gh` CLI must be authenticated locally (`gh auth login`) for the script
   to dispatch the workflow.

No `TWINE_USERNAME`/`TWINE_PASSWORD` is needed for the GHA path — Trusted
Publishing uses OIDC. The local `.env` is only relevant for `--build-only`
mode (which doesn't upload anyway).
