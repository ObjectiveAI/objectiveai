//! Python execution via the embedded WASI RustPython interpreter.
//!
//! [`Python`] holds the JIT-compiled interpreter module; it is built
//! by [`Python::initialize`], lazily, via [`crate::context::Context::python`].
//! Initialization resolves the machine-wide compiled-artifact cache at
//! `<bin>/cache/rustpython-<hash>.cwasm`, where `<hash>` is the
//! build-time xxhash3_128 of the embedded blob
//! ([`crate::python_wasm::RUSTPYTHON_WASM_HASH`]). Creation is
//! serialized by a BIN lock (`<bin>/locks`, key = the hash — machine-
//! wide): probe → `wait_acquire` → re-probe →
//! decompress + JIT + serialize + atomic publish (tmp + rename) →
//! explicit release. A stale-wasmtime or corrupt artifact fails
//! deserialization and reads as a cache miss, so the cache self-heals
//! by rebuilding; the embedded blob is always the source of truth.
//!
//! Execution runs `rustpython -c <wrapped code>` per call in a fresh
//! wasmtime instance with NO preopened directories, environment
//! variables, or sockets. stdout/stderr are captured in-memory;
//! `--python-file` sources are read host-side and passed in as code, so
//! the sandbox never needs filesystem access.
//!
//! The ONE host reach is the `objectiveai` native module's
//! `objectiveai.execute(argv) -> list`, wired in by [`add_objectiveai_host`]:
//! Python cannot touch the host directly, but it can ASK the host to run a
//! CLI command in-process (the host runs the CLI's own dispatcher against the
//! call's context pair and returns the streamed output) — the same mediated
//! posture as a plugin's nested command. Recursion is allowed: a command run
//! this way may itself run Python.
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

use futures::StreamExt;
use tokio::runtime::Handle;
use wasmtime::{Caller, Engine, Linker, Module, Store};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{I32Exit, WasiCtxBuilder};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// Store data for a Python run: the WASI ctx plus what the `objectiveai.execute`
/// host import needs — the [`GlobalContext`]/[`ScopedContext`] pair to
/// dispatch against (the scope carries the CALLER's identity through
/// nested dispatch) and a tokio runtime [`Handle`] to drive the async
/// dispatch from the (synchronous, `spawn_blocking`) wasm thread.
/// `result` stashes the bytes a `host_execute` call produced so the
/// paired `host_result` call can copy them into a right-sized guest
/// buffer WITHOUT re-running the (possibly side-effecting) command.
struct PyHostState {
    wasi: WasiP1Ctx,
    global: GlobalContext,
    scoped: ScopedContext,
    handle: Handle,
    result: Vec<u8>,
}

/// Top bit of `host_execute`'s return value: set ⇒ the stashed bytes are an
/// error message (the guest raises), clear ⇒ they're the JSON result. `usize`
/// is 32-bit on wasm32, matching the guest-side constant.
const ERR_BIT: u32 = 1 << 31;

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
/// [`crate::context::GlobalContext`] clones via its `OnceCell`.
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
    /// into `T`. `input` (any serializable value, or `None`) is exposed
    /// to the code as a global `input` variable.
    ///
    /// The harness envelope carries `eval` (bare trailing expression
    /// value) and `stdout` (captured prints); `eval` is tried first,
    /// then `stdout`. `Ok(None)` means the script produced NO output
    /// (no trailing expression and nothing printed) — callers that
    /// require a value treat that as an error; output transforms treat
    /// it as "skip".
    pub async fn exec_code<I, T>(
        &self,
        global: &GlobalContext, scoped: &ScopedContext,
        code: &str,
        input: Option<I>,
    ) -> Result<Option<T>, Error>
    where
        I: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let input = match input {
            Some(i) => serde_json::to_value(i).map_err(Error::InlineJson)?,
            None => serde_json::Value::Null,
        };
        let wrapped = wrap_code(code, &input);
        let engine = self.engine.clone();
        let module = self.module.clone();
        // The `objectiveai.execute` host import dispatches CLI commands in
        // process; it needs the context pair + a runtime handle to drive async
        // dispatch from the (blocking) wasm thread. Capture the handle here,
        // where we are on the runtime, and clone the cheap (Arc-backed) pair.
        let global = global.clone();
        let scoped = scoped.clone();
        let handle = Handle::current();
        // Wasm execution is blocking CPU work.
        let raw = tokio::task::spawn_blocking(move || {
            run_blocking(&engine, &module, &wrapped, global, scoped, handle)
        })
        .await
        .map_err(|e| Error::PythonWasm(format!("python task join: {e}")))??;
        parse_envelope(&raw)
    }

    /// Execute a Python script file (read host-side) and deserialize
    /// its output as JSON into `T`. See [`Self::exec_code`] for the
    /// `input` and `Ok(None)` semantics.
    pub async fn exec_file<I, T>(
        &self,
        global: &GlobalContext, scoped: &ScopedContext,
        path: &Path,
        input: Option<I>,
    ) -> Result<Option<T>, Error>
    where
        I: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let code = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Error::PythonFileRead(path.to_path_buf(), e))?;
        self.exec_code(global, scoped, &code, input).await
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
fn run_blocking(
    engine: &Engine,
    module: &Module,
    code: &str,
    global: GlobalContext,
    scoped: ScopedContext,
    handle: Handle,
) -> Result<String, Error> {
    let mut linker: Linker<PyHostState> = Linker::new(engine);
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s: &mut PyHostState| &mut s.wasi)
        .map_err(wasm_err)?;
    add_objectiveai_host(&mut linker)?;

    let stdout = MemoryOutputPipe::new(MAX_OUTPUT_BYTES);
    let stderr = MemoryOutputPipe::new(MAX_OUTPUT_BYTES);
    // No preopened dirs, no env, no inherited stdio — the sandbox. The ONLY
    // host reach is the `objectiveai.execute` import wired in above, which the
    // host fully mediates: it runs the CLI's own dispatcher against the pair.
    let wasi = WasiCtxBuilder::new()
        .args(&["rustpython", "-c", code])
        .stdout(stdout.clone())
        .stderr(stderr.clone())
        .build_p1();
    let mut store = Store::new(
        engine,
        PyHostState {
            wasi,
            global,
            scoped,
            handle,
            result: Vec::new(),
        },
    );

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

/// Wire the `objectiveai` host module into the linker. Two imports, so a
/// (possibly side-effecting) command runs exactly once even when the guest has
/// to resize its output buffer:
///   `host_execute(argv_ptr, argv_len) -> len|ERR_BIT` — run argv (a JSON
///       `list[str]`) through the CLI dispatcher, stash the result (JSON array,
///       or an error message with [`ERR_BIT`] set), and return its byte length.
///   `host_result(out_ptr)` — copy the stash into the guest buffer and clear it.
fn add_objectiveai_host(linker: &mut Linker<PyHostState>) -> Result<(), Error> {
    linker
        .func_wrap(
            "objectiveai",
            "host_execute",
            |mut caller: Caller<'_, PyHostState>, argv_ptr: u32, argv_len: u32| -> u32 {
                // `--no-objectiveai`: the host call raises in the guest instead
                // of dispatching. Checked first — don't even read guest memory.
                if caller.data().scoped.no_objectiveai {
                    return stash_error(
                        &mut caller,
                        "objectiveai.execute is disabled (--no-objectiveai)",
                    );
                }
                let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) else {
                    return stash_error(&mut caller, "objectiveai.execute: guest exports no memory");
                };
                // Copy the argv JSON out; scope the immutable memory borrow so
                // the later `data_mut()` is free.
                let req = {
                    let data = memory.data(&caller);
                    let start = argv_ptr as usize;
                    let end = start.saturating_add(argv_len as usize);
                    match data.get(start..end) {
                        Some(bytes) => bytes.to_vec(),
                        None => {
                            return stash_error(
                                &mut caller,
                                "objectiveai.execute: argv pointer/length out of bounds",
                            );
                        }
                    }
                };
                let global = caller.data().global.clone();
                let scoped = caller.data().scoped.clone();
                let handle = caller.data().handle.clone();
                match run_request(global, scoped, handle, &req) {
                    Ok(bytes) => {
                        let len = bytes.len() as u32;
                        caller.data_mut().result = bytes;
                        len
                    }
                    Err(msg) => stash_error(&mut caller, &msg),
                }
            },
        )
        .map_err(wasm_err)?;
    linker
        .func_wrap(
            "objectiveai",
            "host_result",
            |mut caller: Caller<'_, PyHostState>, out_ptr: u32| {
                let result = std::mem::take(&mut caller.data_mut().result);
                if let Some(memory) = caller.get_export("memory").and_then(|e| e.into_memory()) {
                    let _ = memory.write(&mut caller, out_ptr as usize, &result);
                }
            },
        )
        .map_err(wasm_err)?;
    Ok(())
}

/// Stash `msg` as the call result and return its length with [`ERR_BIT`] set so
/// the guest raises it as an exception.
fn stash_error(caller: &mut Caller<'_, PyHostState>, msg: &str) -> u32 {
    let bytes = msg.as_bytes().to_vec();
    let len = bytes.len() as u32;
    caller.data_mut().result = bytes;
    len | ERR_BIT
}

/// Service an `objectiveai.execute` request from the (synchronous, blocking)
/// wasm thread. The dispatch is `spawn`ed onto the runtime's workers and the
/// result is awaited over a std channel — NOT driven with `Handle::block_on` on
/// this `spawn_blocking` thread, which deadlocks. `Ok(json_bytes)` / `Err(msg)`.
fn run_request(
    global: GlobalContext,
    scoped: ScopedContext,
    handle: Handle,
    req: &[u8],
) -> Result<Vec<u8>, String> {
    let req = req.to_vec();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    handle.spawn(async move {
        let _ = tx.send(run_request_async(global, scoped, req).await);
    });
    rx.recv()
        .map_err(|e| format!("objectiveai.execute: dispatch worker dropped: {e}"))?
}

/// The async half of [`run_request`], run on the runtime's workers. `req` is a
/// JSON value that is either a single argv (`list[str]`) → run it and return a
/// JSON array of the streamed items; or a batch (`list[list[str]]`) → run each
/// argv IN PARALLEL and return a JSON array of those arrays. An empty list is an
/// empty batch (`[]`). A batch is all-or-nothing: any command's failure becomes
/// the call's error (after all commands have run). Recursion is allowed: a
/// command run here may itself run Python, which may call `execute` again.
async fn run_request_async(
    global: GlobalContext,
    scoped: ScopedContext,
    req: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let arr = match serde_json::from_slice::<serde_json::Value>(&req) {
        Ok(serde_json::Value::Array(arr)) => arr,
        Ok(_) => {
            return Err(
                "objectiveai.execute: expected a list[str] (argv) or list[list[str]] (batch)"
                    .to_string(),
            );
        }
        Err(e) => return Err(format!("objectiveai.execute: invalid request: {e}")),
    };
    // Batch ⇔ empty, or the first element is itself a list. A single argv is a
    // non-empty list of strings.
    let is_batch = arr.first().map_or(true, serde_json::Value::is_array);
    if is_batch {
        let argvs: Vec<Vec<String>> = serde_json::from_value(serde_json::Value::Array(arr))
            .map_err(|e| {
                format!("objectiveai.execute: invalid batch (expected list[list[str]]): {e}")
            })?;
        // Run the argvs in parallel on the runtime's workers; await all.
        let tasks: Vec<_> = argvs
            .into_iter()
            .map(|argv| {
                let global = global.clone();
                let scoped = scoped.clone();
                tokio::spawn(async move { run_one_argv(&global, &scoped, argv).await })
            })
            .collect();
        let results = futures::future::join_all(tasks).await;
        let mut out: Vec<Vec<objectiveai_sdk::cli::command::ResponseItem>> =
            Vec::with_capacity(results.len());
        for result in results {
            // Outer: JoinError (the task panicked). Inner: the command's error.
            out.push(result.map_err(|e| format!("objectiveai.execute: task panicked: {e}"))??);
        }
        serde_json::to_vec(&out).map_err(|e| format!("objectiveai.execute: serialize: {e}"))
    } else {
        let argv: Vec<String> = serde_json::from_value(serde_json::Value::Array(arr))
            .map_err(|e| format!("objectiveai.execute: invalid argv (expected list[str]): {e}"))?;
        let items = run_one_argv(&global, &scoped, argv).await?;
        serde_json::to_vec(&items).map_err(|e| format!("objectiveai.execute: serialize: {e}"))
    }
}

/// Parse + dispatch a single argv against the context pair, collecting
/// the streamed `ResponseItem`s. `Err(message)` on a parse or dispatch
/// error.
async fn run_one_argv(
    global: &GlobalContext,
    scoped: &ScopedContext,
    argv: Vec<String>,
) -> Result<Vec<objectiveai_sdk::cli::command::ResponseItem>, String> {
    let request = objectiveai_sdk::cli::command::parse_request(&argv).map_err(|e| match e {
        objectiveai_sdk::cli::command::ParseError::Clap(e) => format!("objectiveai.execute: {e}"),
        objectiveai_sdk::cli::command::ParseError::FromArgs(e) => {
            format!("objectiveai.execute: {e:?}")
        }
    })?;
    let mut stream = crate::command::command::execute(global, scoped, request)
        .await
        .map_err(|e| format!("objectiveai.execute: {e}"))?;
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item.map_err(|e| format!("objectiveai.execute: {e}"))?);
    }
    Ok(items)
}

fn wasm_err(e: wasmtime::Error) -> Error {
    Error::PythonWasm(format!("{e:#}"))
}

/// Parse the harness envelope: `eval` first (non-null means the last
/// statement was a bare expression), then the captured `stdout`.
/// `Ok(None)` means no output at all — a null `eval` AND empty stdout.
fn parse_envelope<T: serde::de::DeserializeOwned>(raw: &str) -> Result<Option<T>, Error> {
    let envelope: HarnessOutput = serde_json::from_str(raw)
        .map_err(|e| Error::PythonHarnessBroken(e.to_string()))?;

    let eval_err = if !envelope.eval.is_null() {
        let eval_str = envelope.eval.to_string();
        let mut de = serde_json::Deserializer::from_str(&eval_str);
        match serde_path_to_error::deserialize(&mut de) {
            Ok(result) => return Ok(Some(result)),
            Err(e) => Some(e),
        }
    } else {
        None
    };

    // No usable `eval`. Empty stdout ⇒ genuinely no output (`None`);
    // an `eval` that was present but failed to deserialize is a real
    // error, not "no output".
    if envelope.stdout.trim().is_empty() {
        return match eval_err {
            Some(e) => Err(Error::PythonDeserialize(e)),
            None => Ok(None),
        };
    }

    let mut de = serde_json::Deserializer::from_str(&envelope.stdout);
    match serde_path_to_error::deserialize(&mut de) {
        Ok(result) => Ok(Some(result)),
        Err(stdout_err) => Err(Error::PythonDeserialize(eval_err.unwrap_or(stdout_err))),
    }
}

/// Wrap user code in the envelope harness: detect a bare trailing
/// expression with `ast`, capture prints into a StringIO, emit
/// `{"eval": ..., "stdout": "..."}` as the final stdout line. The
/// user code is base64-embedded purely so arbitrary source text
/// survives inclusion in the harness source. No defensive measures —
/// see the module docs.
fn wrap_code(code: &str, input: &serde_json::Value) -> String {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(code);
    // Serializing a `Value` is infallible; fall back to JSON null.
    let input_json = serde_json::to_string(input).unwrap_or_else(|_| "null".to_string());
    let encoded_input = base64::engine::general_purpose::STANDARD.encode(input_json);
    // `input` is exposed to user code as a global, decoded from JSON at
    // runtime (JSON literals aren't Python literals — `true`/`null`).
    // `objectiveai` (the native host module) is also injected as a global so
    // user code can call `objectiveai.execute(argv)` WITHOUT an explicit
    // `import objectiveai` — the same builtin-like posture as `input`.
    format!(
        r#"
import ast as __oai_ast, json as __oai_json, sys as __oai_sys, io as __oai_io, base64 as __oai_b64, os as __oai_os
import objectiveai as __oai_objectiveai
__oai_real_stdout = __oai_sys.stdout
__oai_tree = __oai_ast.parse(__oai_b64.b64decode("{encoded}").decode())
__oai_input = __oai_json.loads(__oai_b64.b64decode("{encoded_input}").decode())
__oai_capture = __oai_io.StringIO()
__oai_sys.stdout = __oai_capture
__oai_eval = None
__oai_globals = {{"__name__": "__main__", "__builtins__": __builtins__, "input": __oai_input, "objectiveai": __oai_objectiveai}}
if __oai_tree.body and isinstance(__oai_tree.body[-1], __oai_ast.Expr):
    __oai_last = __oai_tree.body.pop()
    exec(compile(__oai_tree, "<inline>", "exec"), __oai_globals)
    __oai_eval = eval(compile(__oai_ast.Expression(__oai_last.value), "<inline>", "eval"), __oai_globals)
else:
    exec(compile(__oai_tree, "<inline>", "exec"), __oai_globals)
# Restore the REAL captured stdout (not sys.__stdout__, which is None in
# this WASI build — restoring it would leave a None stream that the
# interpreter's shutdown flush trips over: "Exception ignored in: None:
# 'NoneType' object has no attribute 'flush'", a non-zero exit).
__oai_sys.stdout = __oai_real_stdout
print(__oai_json.dumps({{"eval": __oai_eval, "stdout": __oai_capture.getvalue()}}))
# Flush and hard-exit before finalization runs — skips the buggy
# shutdown flush entirely while preserving the emitted envelope.
__oai_sys.stdout.flush()
__oai_os._exit(0)
"#
    )
}
