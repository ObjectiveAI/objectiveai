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
const RECIPE: &str = "v1";

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
        return Ok(());
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
    })
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

/// Build rustpython for wasm32-wasip1, compress, and publish the blob
/// atomically. Caller holds the cache lock.
fn build_and_publish(cache: &Path, blob: &Path) -> std::io::Result<()> {
    clean_stale_tmp(cache);
    ensure_wasip1_target();
    run_cargo_install(cache)?;

    // `cargo install --target wasm32-wasip1` is expected to copy the
    // bin into `<root>/bin/`; the explicit CARGO_TARGET_DIR gives us a
    // deterministic fallback location if it doesn't.
    let candidates = [
        cache.join("install/bin/rustpython.wasm"),
        cache.join("target/wasm32-wasip1/release/rustpython.wasm"),
    ];
    let wasm_path = candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            std::io::Error::other(format!(
                "cargo install succeeded but rustpython.wasm was not found at {} or {}",
                candidates[0].display(),
                candidates[1].display(),
            ))
        })?;
    let wasm = std::fs::read(wasm_path)?;
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
    for dir in ["install", "target"] {
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

/// Build the pinned rustpython bin for wasm32-wasip1 via a nested
/// `cargo install`.
///
/// The feature recipe is FIXED at `--no-default-features --features
/// freeze-stdlib`, for two reasons:
///
/// - **Determinism**: one blob filename ⇔ one feature set on every
///   machine. An adaptive ladder would let the same cache name carry
///   different contents depending on the host.
/// - **No host C toolchain**: default features pull `libz-sys`,
///   whose C build for a wasm target requires clang; the minimal
///   recipe builds with nothing beyond rustc + the wasip1 target.
///   (Cost: no `zlib` module inside the sandbox.)
///
/// The attempt ladder varies ONLY `--locked` (first success wins):
/// the crate's bundled lockfile is preferred for reproducibility but
/// may be absent or stale against a newer toolchain.
fn run_cargo_install(cache: &Path) -> std::io::Result<()> {
    let attempts: [&[&str]; 2] = [
        &["--locked", "--no-default-features", "--features", "freeze-stdlib"],
        &["--no-default-features", "--features", "freeze-stdlib"],
    ];
    let mut failures = String::new();
    for extra in attempts {
        let status = cargo_install_once(cache, extra)?;
        if status.success() {
            return Ok(());
        }
        failures.push_str(&format!("\n  cargo install {extra:?} -> {status}"));
    }
    Err(std::io::Error::other(format!(
        "all cargo install attempts for rustpython {RUSTPYTHON_VERSION} \
         (wasm32-wasip1) failed:{failures}\nsee the build output above for details",
    )))
}

/// One nested `cargo install` invocation with strict env hygiene.
///
/// Rules (each guards against a real failure mode):
///
/// - Strip every inherited `CARGO_*`/`RUST*` var except `CARGO`,
///   `CARGO_HOME`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN` — kills
///   `RUSTC(_WRAPPER)`, `CARGO_BUILD_*`, `CARGO_PROFILE_*`,
///   incremental flags, and target-specific overrides meant for the
///   parent build, while keeping the registry cache and toolchain
///   selection.
/// - Strip `CARGO_MAKEFLAGS`/`MAKEFLAGS`/`MFLAGS` — the parent's
///   jobserver fds are not usable from the grandchild on Unix.
/// - SET `CARGO_TARGET_DIR=<cache>/target` and `RUSTFLAGS=""` rather
///   than merely removing them: env beats config files, so a user
///   `~/.cargo/config.toml` with `build.target-dir` can't point the
///   child at the parent's locked target dir (deadlock) and config
///   rustflags can't leak into the wasm build.
/// - Child stdout is piped and re-emitted on OUR stderr: anything a
///   build script prints to stdout is interpreted by cargo as
///   directives.
fn cargo_install_once(
    cache: &Path,
    extra_args: &[&str],
) -> std::io::Result<std::process::ExitStatus> {
    let cargo = std::env::var_os("CARGO").expect("cargo sets CARGO");
    let mut cmd = std::process::Command::new(cargo);
    cmd.args([
        "install",
        "rustpython",
        "--version",
        RUSTPYTHON_VERSION,
        "--target",
        "wasm32-wasip1",
        "--force",
    ]);
    cmd.arg("--root").arg(cache.join("install"));
    cmd.args(extra_args);
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
    Ok(output.status)
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
