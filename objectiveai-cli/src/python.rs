//! Python execution via the embedded WASI RustPython interpreter.
//!
//! [`Python`] holds the JIT-compiled interpreter module; it is built
//! by [`Python::initialize`], lazily, via [`crate::context::Context::python`].
//! Initialization resolves the machine-wide compiled-artifact cache at
//! `<bin>/cache/rustpython-<hash>.cwasm`, where `<hash>` is the
//! build-time xxhash3_128 of the embedded blob
//! ([`crate::python_wasm::RUSTPYTHON_WASM_HASH`]). Creation is
//! serialized by a BIN lock (`<bin>/locks`, key = the hash — machine-
//! wide, like the api spawn lock): probe → `wait_acquire` → re-probe →
//! decompress + JIT + serialize + atomic publish (tmp + rename) →
//! explicit release. A stale-wasmtime or corrupt artifact fails
//! deserialization and reads as a cache miss, so the cache self-heals
//! by rebuilding; the embedded blob is always the source of truth.
//!
//! Execution runs `rustpython -c <wrapped code>` per call in a fresh
//! wasmtime instance with NO preopened directories, environment
//! variables, or sockets — Python code cannot reach the host.
//! stdout/stderr are captured in-memory; `--python-file` sources are
//! read host-side and passed in as code, so the sandbox never needs
//! filesystem access.
//!
//! User code is wrapped in a harness that uses `ast` to detect a bare
//! trailing expression. The harness prints a JSON envelope with `eval`
//! (the expression result, or null) and `stdout` (captured print
//! output) as the process's last stdout line; the Rust side tries
//! `eval` first, falling back to `stdout`. The harness has NO
//! safeguards: user code that breaks envelope emission (rebinding
//! `json`, swapping `sys.stdout`, calling `sys.exit(0)`) makes the
//! run fail with [`PythonHarnessBroken`] or [`PythonException`] —
//! that's on the caller.
//!
//! [`PythonHarnessBroken`]: crate::error::Error::PythonHarnessBroken
//! [`PythonException`]: crate::error::Error::PythonException

use std::path::{Path, PathBuf};

use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{I32Exit, WasiCtxBuilder};

use crate::error::Error;

/// Cap on captured stdout/stderr per execution.
const MAX_OUTPUT_BYTES: usize = 256 * 1024 * 1024;

/// The JSON envelope produced by the Python harness.
#[derive(serde::Deserialize)]
struct HarnessOutput {
    eval: serde_json::Value,
    stdout: String,
}

/// A ready-to-execute WASI RustPython interpreter. Cheap to clone
/// internally (`Engine`/`Module` are Arc-backed); shared across
/// [`crate::context::Context`] clones via its `OnceCell`.
pub struct Python {
    engine: Engine,
    module: Module,
}

impl Python {
    /// Load (or create, under the bin lock) the JIT-compiled
    /// interpreter. See the module docs for the full dance.
    pub async fn initialize(bin_dir: PathBuf) -> Result<Self, Error> {
        let engine = Engine::new(&wasmtime::Config::new()).map_err(wasm_err)?;
        let cache_dir = bin_dir.join("cache");
        let artifact = cache_dir.join(format!(
            "rustpython-{}.cwasm",
            crate::python_wasm::RUSTPYTHON_WASM_HASH
        ));

        // Fast path: artifact present and loadable.
        if let Some(module) = try_load(&engine, &artifact).await {
            return Ok(Self { engine, module });
        }

        let claim = objectiveai_sdk::lockfile::wait_acquire(
            &bin_dir.join("locks"),
            crate::python_wasm::RUSTPYTHON_WASM_HASH,
            &format!("pid {}", std::process::id()),
        )
        .await
        .map_err(|e| Error::PythonWasm(format!("bin lock: {e}")))?;
        let result = create_under_lock(&engine, &cache_dir, &artifact).await;
        // Explicit release on the handled paths; on abnormal death the
        // kernel releases the claim (the lockfile module's design).
        claim
            .release()
            .map_err(|e| Error::PythonWasm(format!("bin lock release: {e}")))?;
        Ok(Self {
            engine,
            module: result?,
        })
    }

    /// Execute inline Python code and deserialize its output as JSON
    /// into `T`. The harness envelope carries `eval` (bare trailing
    /// expression value) and `stdout` (captured prints); `eval` is
    /// tried first, then `stdout`.
    pub async fn exec_code<T: serde::de::DeserializeOwned>(
        &self,
        code: &str,
    ) -> Result<T, Error> {
        let wrapped = wrap_code(code);
        let engine = self.engine.clone();
        let module = self.module.clone();
        // Wasm execution is blocking CPU work.
        let raw = tokio::task::spawn_blocking(move || run_blocking(&engine, &module, &wrapped))
            .await
            .map_err(|e| Error::PythonWasm(format!("python task join: {e}")))??;
        parse_envelope(&raw)
    }

    /// Execute a Python script file (read host-side) and deserialize
    /// its output as JSON into `T`.
    pub async fn exec_file<T: serde::de::DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<T, Error> {
        let code = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Error::PythonFileRead(path.to_path_buf(), e))?;
        self.exec_code(&code).await
    }
}

/// Attempt to load the cached artifact. Any failure — missing file,
/// engine-fingerprint mismatch after a wasmtime upgrade, corruption —
/// reads as a cache miss.
async fn try_load(engine: &Engine, artifact: &Path) -> Option<Module> {
    if !tokio::fs::try_exists(artifact).await.unwrap_or(false) {
        return None;
    }
    let engine = engine.clone();
    let path = artifact.to_path_buf();
    tokio::task::spawn_blocking(move || {
        // SAFETY: the artifact is this binary's own `Module::serialize`
        // output published under the bin lock; wasmtime validates the
        // embedded engine fingerprint on load and rejects mismatches,
        // and the path is inside our own layout root.
        unsafe { Module::deserialize_file(&engine, &path) }.ok()
    })
    .await
    .ok()
    .flatten()
}

/// Create and publish the JIT artifact. Caller holds the bin lock.
async fn create_under_lock(
    engine: &Engine,
    cache_dir: &Path,
    artifact: &Path,
) -> Result<Module, Error> {
    // Re-probe under the lock — a concurrent process may have
    // published while we waited.
    if let Some(module) = try_load(engine, artifact).await {
        return Ok(module);
    }

    let (module, serialized) = {
        let engine = engine.clone();
        tokio::task::spawn_blocking(move || -> Result<(Module, Vec<u8>), Error> {
            let wasm = zstd::decode_all(crate::python_wasm::RUSTPYTHON_WASM_ZSTD)
                .map_err(|e| Error::PythonWasm(format!("decompress embedded blob: {e}")))?;
            let module = Module::new(&engine, &wasm).map_err(wasm_err)?;
            let serialized = module.serialize().map_err(wasm_err)?;
            Ok((module, serialized))
        })
        .await
        .map_err(|e| Error::PythonWasm(format!("python jit task join: {e}")))??
    };

    // Publish atomically: tmp + rename, so an incomplete file is never
    // observable at the final path. Best-effort — we already hold a
    // working module, and a missed publish only costs the next process
    // a recompile.
    let publish = async {
        tokio::fs::create_dir_all(cache_dir).await?;
        let tmp = artifact.with_extension(format!("cwasm.tmp-{}", std::process::id()));
        tokio::fs::write(&tmp, &serialized).await?;
        tokio::fs::rename(&tmp, artifact).await
    };
    if publish.await.is_ok() {
        prune_stale(cache_dir, artifact).await;
    }
    Ok(module)
}

/// Remove artifacts from older blobs/wasmtimes and dead tmp files.
/// Best-effort; safe because the caller holds the bin lock.
async fn prune_stale(cache_dir: &Path, current: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(cache_dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path == current {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("rustpython-") && (name.ends_with(".cwasm") || name.contains(".cwasm.tmp-")) {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

/// One sandboxed `rustpython -c <code>` run, blocking.
///
/// Outcome mapping:
/// - clean return or `exit(0)` → captured stdout (the envelope)
/// - nonzero exit → [`Error::PythonException`] with stderr (the
///   traceback), matching Python's CLI behavior
/// - any other trap (stack exhaustion, runtime bug) →
///   [`Error::PythonWasm`]
fn run_blocking(engine: &Engine, module: &Module, code: &str) -> Result<String, Error> {
    let mut linker: Linker<WasiP1Ctx> = Linker::new(engine);
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |t| t).map_err(wasm_err)?;

    let stdout = MemoryOutputPipe::new(MAX_OUTPUT_BYTES);
    let stderr = MemoryOutputPipe::new(MAX_OUTPUT_BYTES);
    // No preopened dirs, no env, no inherited stdio — the sandbox.
    let wasi = WasiCtxBuilder::new()
        .args(&["rustpython", "-c", code])
        .stdout(stdout.clone())
        .stderr(stderr.clone())
        .build_p1();
    let mut store = Store::new(engine, wasi);

    let outcome = linker
        .instantiate(&mut store, module)
        .and_then(|instance| instance.get_typed_func::<(), ()>(&mut store, "_start"))
        .and_then(|start| start.call(&mut store, ()));
    drop(store);

    let exited_clean = match outcome {
        Ok(()) => true,
        Err(e) => match e.downcast_ref::<I32Exit>() {
            Some(exit) if exit.0 == 0 => true,
            Some(_) => {
                let stderr = String::from_utf8_lossy(&stderr.contents()).into_owned();
                return Err(Error::PythonException(stderr));
            }
            None => {
                let stderr = String::from_utf8_lossy(&stderr.contents()).into_owned();
                return Err(Error::PythonWasm(format!("{e:#}; stderr:\n{stderr}")));
            }
        },
    };
    debug_assert!(exited_clean);
    Ok(String::from_utf8_lossy(&stdout.contents()).into_owned())
}

fn wasm_err(e: wasmtime::Error) -> Error {
    Error::PythonWasm(format!("{e:#}"))
}

/// Parse the harness envelope: `eval` first (non-null means the last
/// statement was a bare expression), then the captured `stdout`.
fn parse_envelope<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, Error> {
    let envelope: HarnessOutput = serde_json::from_str(raw)
        .map_err(|e| Error::PythonHarnessBroken(e.to_string()))?;

    let eval_err = if !envelope.eval.is_null() {
        let eval_str = envelope.eval.to_string();
        let mut de = serde_json::Deserializer::from_str(&eval_str);
        match serde_path_to_error::deserialize(&mut de) {
            Ok(result) => return Ok(result),
            Err(e) => Some(e),
        }
    } else {
        None
    };

    let mut de = serde_json::Deserializer::from_str(&envelope.stdout);
    match serde_path_to_error::deserialize(&mut de) {
        Ok(result) => Ok(result),
        Err(stdout_err) => Err(Error::PythonDeserialize(eval_err.unwrap_or(stdout_err))),
    }
}

/// Wrap user code in the envelope harness: detect a bare trailing
/// expression with `ast`, capture prints into a StringIO, emit
/// `{"eval": ..., "stdout": "..."}` as the final stdout line. The
/// user code is base64-embedded purely so arbitrary source text
/// survives inclusion in the harness source. No defensive measures —
/// see the module docs.
fn wrap_code(code: &str) -> String {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(code);
    format!(
        r#"
import ast as __oai_ast, json as __oai_json, sys as __oai_sys, io as __oai_io, base64 as __oai_b64
__oai_tree = __oai_ast.parse(__oai_b64.b64decode("{encoded}").decode())
__oai_capture = __oai_io.StringIO()
__oai_sys.stdout = __oai_capture
__oai_eval = None
__oai_globals = {{"__name__": "__main__", "__builtins__": __builtins__}}
if __oai_tree.body and isinstance(__oai_tree.body[-1], __oai_ast.Expr):
    __oai_last = __oai_tree.body.pop()
    exec(compile(__oai_tree, "<inline>", "exec"), __oai_globals)
    __oai_eval = eval(compile(__oai_ast.Expression(__oai_last.value), "<inline>", "eval"), __oai_globals)
else:
    exec(compile(__oai_tree, "<inline>", "exec"), __oai_globals)
__oai_sys.stdout = __oai_sys.__stdout__
print(__oai_json.dumps({{"eval": __oai_eval, "stdout": __oai_capture.getvalue()}}))
"#
    )
}
