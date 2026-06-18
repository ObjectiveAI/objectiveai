# objectiveai-sdk-rs-pyo3

PyO3 bindings for ObjectiveAI.

This crate is consumed by the sibling `objectiveai-py` package via maturin's
`manifest-path` setting (in `objectiveai-sdk-py/pyproject.toml`). The compiled
extension is bundled into the `objectiveai-sdk` PyPI wheel as `objectiveai_sdk._pyo3`,
not published as a separate distribution.

## Layout

- `Cargo.toml` — workspace member, `[lib] crate-type = ["cdylib"]`,
  `[lib] name = "_pyo3"`, `pyo3 features = ["abi3-py310"]`.
- `src/lib.rs` — `#[pymodule] fn _pyo3(...)` exposing all `#[pyfunction]`
  entry points to Python.

## Building

There is no standalone build for this crate; it is built as part of the
`objectiveai-py` package:

```bash
bash objectiveai-sdk-py/build.sh             # local dev (maturin develop)
```

Cross-platform wheels are built and published to PyPI by the `Release`
GitHub Actions workflow on a version bump.

`maturin` reads `objectiveai-sdk-py/pyproject.toml`, follows `manifest-path` to
this `Cargo.toml`, and produces a wheel containing the pure-Python sources
plus the compiled extension.
