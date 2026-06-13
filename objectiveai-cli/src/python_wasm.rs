//! The embedded WASI RustPython interpreter, as produced by build.rs:
//! `rustpython` (pinned version, `--no-default-features --features
//! freeze-stdlib`) compiled for wasm32-wasip1 and zstd-compressed.
//! [`crate::python`] decompresses and JIT-compiles it on first use.

/// The compressed `rustpython.wasm` (zstd).
pub(crate) static RUSTPYTHON_WASM_ZSTD: &[u8] =
    include_bytes!(env!("RUSTPYTHON_WASM_ZSTD_PATH"));

/// Build-time xxhash3_128 (hex) of [`RUSTPYTHON_WASM_ZSTD`]. Keys the
/// machine-wide JIT artifact cache and its bin lock: one hash ⇔ one
/// exact interpreter blob.
pub(crate) const RUSTPYTHON_WASM_HASH: &str = env!("RUSTPYTHON_WASM_HASH");
