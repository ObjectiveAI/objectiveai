//! objectiveai-cli build script.
//!
//! Two jobs:
//!
//! 1. Set the main-thread stack size to 16 MB on every supported
//!    platform.
//! 2. Produce and embed the WASI RustPython blob: build the pinned
//!    `rustpython` crate for `wasm32-wasip1`, zstd-compress it, cache
//!    it under `objectiveai-cli/.cache/`, and point
//!    `RUSTPYTHON_WASM_ZSTD_PATH` at it for the `include_bytes!` in
//!    `src/python_wasm.rs`.
//!
//! The blob build is expensive (minutes, network), so the cache is
//! shared across target dirs and profiles, and guarded by the SDK's
//! cross-process lockfile: probe → [`objectiveai_sdk::lockfile::wait_acquire`]
//! → re-probe under the lock → build + publish atomically (tmp file +
//! fsync + rename) → explicit [`LockClaim::release`]. A killed build
//! never wedges the lock — process death releases it (the lockfile
//! module's design).
//!
//! [`LockClaim::release`]: objectiveai_sdk::lockfile::LockClaim::release

use std::path::{Path, PathBuf};

/// Pinned RustPython version. 0.5.0 is the oldest release that
/// compiles for wasm32-wasip1 on a current toolchain (0.4.0's
/// rustpython-vm uses libc constants that no longer exist for WASI).
const RUSTPYTHON_VERSION: &str = "0.5.0";

/// Build-recipe tag. Bump whenever the wasm build flags change: the
/// cache filename rolls over and the old blob is simply orphaned —
/// the final path is never rebuilt in place.
///
/// v4: build the custom `objectiveai-rustpython-wasm` crate (stock rustpython
/// 0.5.0 + the `objectiveai` native module exposing `objectiveai.execute`)
/// instead of `cargo install rustpython`. The feature set is unchanged
/// (freeze-stdlib, stdio, host_env — now declared in that crate's Cargo.toml).
/// v5: `objectiveai.execute` is polymorphic — a single argv (`list[str]`) or a
/// parallel batch (`list[list[str]]`), output shape mirroring the input. The
/// wasm source changed, so the cache filename must roll (the blob is cached by
/// name, not content).
const RECIPE: &str = "v5";

fn main() {
    set_stack_size();
    if let Err(e) = embed_rustpython_wasm() {
        panic!("failed to provision the RustPython wasm blob: {e}");
    }
}

/// `rustpython-<version>-<recipe>.wasm.zst` — presence at the final
/// cache path ⇔ exactly this version + recipe was built completely.
fn blob_filename() -> String {
    format!("rustpython-{RUSTPYTHON_VERSION}-{RECIPE}.wasm.zst")
}

/// Ensure the compressed blob exists in the cache and emit the
/// directives that embed it.
fn embed_rustpython_wasm() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DOCS_RS");

    // docs.rs builds run offline — the nested cargo install can never
    // succeed there, and docs only need the crate to compile. Embed
    // an empty stub instead.
    if std::env::var_os("DOCS_RS").is_some() {
        let stub = out_dir().join(blob_filename());
        std::fs::write(&stub, [])?;
        println!("cargo:rustc-env=RUSTPYTHON_WASM_ZSTD_PATH={}", stub.display());
        emit_blob_hash(&stub)?;
        return Ok(());
    }

    let cache = resolve_cache_dir();
    std::fs::create_dir_all(&cache)?;
    let blob = cache.join(blob_filename());
    println!("cargo:rerun-if-changed={}", blob.display());
    println!("cargo:rustc-env=RUSTPYTHON_WASM_ZSTD_PATH={}", blob.display());

    // Fast path: a complete blob is only ever observable at the final
    // path (publish is tmp + rename under the lock).
    if blob.exists() {
        return emit_blob_hash(&blob);
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let claim = objectiveai_sdk::lockfile::wait_acquire(
            &cache.join("locks"),
            &blob_filename(),
            &format!("pid {}", std::process::id()),
        )
        .await?;
        // Re-probe under the lock — a concurrent build may have
        // published while we waited.
        let result = if blob.exists() {
            Ok(())
        } else {
            build_and_publish(&cache, &blob)
        };
        // Explicit release on the handled paths; on panic/abort the
        // claim is released by process death.
        claim.release()?;
        result
    })?;
    emit_blob_hash(&blob)
}

/// Where the blob cache lives.
///
/// - In the workspace checkout (detected by the sibling SDK source
///   being present): `objectiveai-cli/.cache/`, shared across every
///   target dir, profile, and concurrent cargo invocation.
/// - Anywhere else (registry builds via `cargo install`, `cargo
///   package` verify): under `OUT_DIR` — never write into the
///   registry source checkout.
fn resolve_cache_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
    );
    if manifest_dir
        .join("../objectiveai-sdk-rs/src/lockfile.rs")
        .exists()
    {
        manifest_dir.join(".cache")
    } else {
        out_dir().join("rustpython-cache")
    }
}

fn out_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"))
}

/// Hash the blob (xxhash3_128, the project's content-addressing hash)
/// and emit `RUSTPYTHON_WASM_HASH`. The runtime keys the machine-wide
/// JIT artifact cache and its bin lock off this constant.
fn emit_blob_hash(blob: &Path) -> std::io::Result<()> {
    let bytes = std::fs::read(blob)?;
    let hash = twox_hash::XxHash3_128::oneshot(&bytes);
    println!("cargo:rustc-env=RUSTPYTHON_WASM_HASH={hash:032x}");
    Ok(())
}

/// Build rustpython for wasm32-wasip1, compress, and publish the blob
/// atomically. Caller holds the cache lock.
fn build_and_publish(cache: &Path, blob: &Path) -> std::io::Result<()> {
    clean_stale_tmp(cache);
    ensure_wasip1_target();
    run_cargo_build(cache)?;

    // `cargo build --target wasm32-wasip1 --release` with our explicit
    // CARGO_TARGET_DIR=<cache>/target writes the bin here. The crate's bin is
    // named `rustpython`, so the artifact keeps the name the runtime invokes.
    let wasm_path = cache.join("target/wasm32-wasip1/release/rustpython.wasm");
    if !wasm_path.exists() {
        return Err(std::io::Error::other(format!(
            "cargo build succeeded but rustpython.wasm was not found at {}",
            wasm_path.display(),
        )));
    }
    let wasm = std::fs::read(&wasm_path)?;
    if !wasm.starts_with(b"\0asm") {
        return Err(std::io::Error::other(format!(
            "{} does not start with the wasm magic bytes",
            wasm_path.display()
        )));
    }

    // Publish atomically: compress to a pid-suffixed tmp in the same
    // directory, fsync, rename. An incomplete file can therefore never
    // exist at the final path.
    let tmp = cache.join(format!("{}.tmp-{}", blob_filename(), std::process::id()));
    {
        let mut tmp_file = std::fs::File::create(&tmp)?;
        zstd::stream::copy_encode(wasm.as_slice(), &mut tmp_file, 19)?;
        tmp_file.sync_all()?;
    }
    rename_with_retry(&tmp, blob)?;

    // Reclaim the ~GB build tree; best-effort.
    for dir in ["target"] {
        let _ = std::fs::remove_dir_all(cache.join(dir));
    }
    Ok(())
}

/// Remove leftover `*.tmp-*` files from builds that died mid-publish.
/// Safe because the caller holds the lock — no live writer owns any
/// tmp file but ours.
fn clean_stale_tmp(cache: &Path) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().contains(".wasm.zst.tmp-") {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `std::fs::rename` with a bounded retry. On Windows a non-Rust
/// handle holder (classically an antivirus scanner) can transiently
/// fail the rename with a sharing violation; correctness does not
/// depend on the retry — the destination never exists in normal flow.
fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last = None;
    for _ in 0..10 {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    Err(last.expect("loop ran"))
}

/// Best-effort `rustup target add wasm32-wasip1` (idempotent). Any
/// failure — including rustup not existing — is downgraded to a
/// warning: the nested cargo build's own error ("can't find crate for
/// `std`") is the real diagnostic, and non-rustup toolchains may have
/// the target preinstalled.
fn ensure_wasip1_target() {
    let mut cmd = std::process::Command::new("rustup");
    cmd.args(["target", "add", "wasm32-wasip1"]);
    if let Some(toolchain) = std::env::var_os("RUSTUP_TOOLCHAIN") {
        cmd.arg("--toolchain").arg(toolchain);
    }
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    match cmd.status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            println!("cargo:warning=rustup target add wasm32-wasip1 exited with {status}");
        }
        Err(e) => {
            println!("cargo:warning=could not run rustup target add wasm32-wasip1: {e}");
        }
    }
}

/// Build the custom `objectiveai-rustpython-wasm` crate for wasm32-wasip1 via a
/// nested `cargo build`, with strict env hygiene. Caller holds the cache lock.
///
/// That crate is the stock rustpython 0.5.0 bin plus the `objectiveai` native
/// module (`objectiveai.execute`); its feature set is FIXED at
/// `freeze-stdlib,stdio,host_env` (declared in its own Cargo.toml). Each is
/// mandatory:
///
/// - **No host C toolchain**: default features pull `ssl-rustls` and `libz-sys`,
///   whose C build for a wasm target requires clang; this recipe builds with
///   nothing beyond rustc + the wasip1 target. (Cost: no `ssl`/`zlib` in the
///   sandbox.)
/// - **`stdio` is mandatory**: without it RustPython's `set_stdio` takes the
///   `#[cfg(not(feature = "stdio"))]` path and sets `sys.stdout`/`sys.stderr` to
///   `None`, so every write is silently discarded and the shutdown flush trips
///   "Exception ignored in: None: 'NoneType' object has no attribute 'flush'".
/// - **`host_env` is mandatory too**: the frozen `io.py` does `from _io import
///   FileIO` at import time, but `_io.FileIO` is `#[cfg(feature = "host_env")]`
///   — so without it `import io` fails. With it on, `stdio` builds
///   `sys.stdout`/`sys.stderr` as real `io.TextIOWrapper`s over fd 1/2 (→ the
///   wasmtime `MemoryOutputPipe`). This does NOT widen the sandbox: the boundary
///   is wasmtime's capability model — the `WasiCtxBuilder` grants no preopened
///   dirs, env vars, or sockets; the only host reach is the single
///   `objectiveai::host_execute` import the embedder wires in for
///   `objectiveai.execute`.
///
/// Env hygiene (each rule guards a real failure mode):
/// - Strip every inherited `CARGO_*`/`RUST*` except `CARGO`, `CARGO_HOME`,
///   `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN` — kills `RUSTC(_WRAPPER)`,
///   `CARGO_BUILD_*`, `CARGO_PROFILE_*`, incremental flags, target overrides.
/// - Strip `CARGO_MAKEFLAGS`/`MAKEFLAGS`/`MFLAGS` — the parent's jobserver fds
///   are not usable from the grandchild on Unix.
/// - SET `CARGO_TARGET_DIR=<cache>/target` and `RUSTFLAGS=""` (env beats config
///   files, so a user `~/.cargo/config.toml` can't redirect the target dir into
///   the parent's locked one or leak rustflags into the wasm build).
/// - Child stdout is piped and re-emitted on OUR stderr (cargo treats build
///   script stdout as directives).
fn run_cargo_build(cache: &Path) -> std::io::Result<()> {
    let manifest = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
    )
    .join("..")
    .join("objectiveai-rustpython-wasm")
    .join("Cargo.toml");

    let cargo = std::env::var_os("CARGO").expect("cargo sets CARGO");
    let mut cmd = std::process::Command::new(cargo);
    cmd.args(["build", "--release", "--target", "wasm32-wasip1", "--manifest-path"]);
    cmd.arg(&manifest);
    cmd.current_dir(cache);

    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        let keep = matches!(
            name.as_ref(),
            "CARGO" | "CARGO_HOME" | "RUSTUP_HOME" | "RUSTUP_TOOLCHAIN"
        );
        if !keep && (name.starts_with("CARGO_") || name.starts_with("RUST")) {
            cmd.env_remove(&key);
        }
    }
    for jobserver in ["CARGO_MAKEFLAGS", "MAKEFLAGS", "MFLAGS"] {
        cmd.env_remove(jobserver);
    }
    cmd.env("CARGO_TARGET_DIR", cache.join("target"));
    cmd.env("RUSTFLAGS", "");

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::inherit());
    let output = cmd.output()?;
    if !output.stdout.is_empty() {
        use std::io::Write;
        let _ = std::io::stderr().write_all(&output.stdout);
    }
    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "cargo build of objectiveai-rustpython-wasm (wasm32-wasip1) failed: {}\n\
             see the build output above for details",
            output.status,
        )))
    }
}

/// Set the main thread stack size to 16 MB for all supported platforms.
fn set_stack_size() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    let flag = match (os.as_str(), env.as_str()) {
        ("windows", "msvc") => "/STACK:16777216",
        ("windows", "gnu") => "-Wl,--stack,16777216",
        ("macos", _) | ("ios", _) => "-Wl,-stack_size,0x1000000",
        ("linux", _) | ("freebsd", _) | ("netbsd", _) | ("openbsd", _) | ("dragonfly", _) => {
            "-Wl,-z,stacksize=16777216"
        }
        _ => return,
    };

    println!("cargo:rustc-link-arg={flag}");
}
