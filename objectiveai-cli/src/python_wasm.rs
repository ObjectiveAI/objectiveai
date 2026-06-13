//! zstd-compressed RustPython wasm32-wasip1 interpreter, embedded at
//! build time. `build.rs` builds the blob from the pinned RustPython
//! version (caching it under `objectiveai-cli/.cache/`) and points
//! `RUSTPYTHON_WASM_ZSTD_PATH` at it.

/// The compressed `rustpython.wasm` (zstd). Decompress before handing
/// to a wasm runtime. Not consumed yet — the WASI execution path
/// lands in a follow-up.
#[allow(dead_code)]
pub(crate) static RUSTPYTHON_WASM_ZSTD: &[u8] =
    include_bytes!(env!("RUSTPYTHON_WASM_ZSTD_PATH"));
