//! Custom WASI RustPython interpreter with the `objectiveai` native module.
//!
//! This is the stock `rustpython` 0.5.0 binary (argv parsing, `-c <code>`,
//! `_start`, exit codes, frozen stdlib via `init_stdlib`) plus one native
//! module — `objectiveai` — exposing:
//!
//!   objectiveai.execute(argv: list[str]) -> list
//!
//! `execute` runs a CLI command **in-process on the host** and returns its
//! streamed output as a list of native Python objects (always a list; one
//! element for unary commands). It is implemented by calling a wasm import
//! `host_execute`/`host_result` that the wasmtime host (the embedder,
//! `objectiveai-cli/src/python.rs`) provides via the "objectiveai" linker
//! module. Recursion is allowed: a command run this way may itself run Python,
//! which may call `objectiveai.execute` again.
//!
//! Built only for wasm32-wasip1 by `objectiveai-cli/build.rs`; not a workspace
//! member (the host import won't link for the host target).

use rustpython::{InterpreterBuilder, InterpreterBuilderExt};

/// Top bit of `host_execute`'s return: set ⇒ the stashed bytes are an error
/// message (raise), clear ⇒ they're the JSON result. The low bits are the byte
/// length. (usize is 32-bit on wasm32, so results are capped at 2 GiB.)
const ERR_BIT: usize = 1 << 31;

// Provided by the wasmtime host via the "objectiveai" linker module. Two calls,
// so a (possibly side-effecting) command runs exactly once and the result is
// copied without re-running on a size mismatch:
//   host_execute(argv_json_ptr, argv_json_len) -> (ERR_BIT? | byte_len)
//       runs argv (a JSON-encoded list[str]) as a CLI command, stashes the
//       result (JSON array on success, error message on failure) host-side, and
//       returns its length (top bit set on error).
//   host_result(out_ptr) copies the stashed bytes into the guest buffer (which
//       the guest sized to the returned length) and clears the stash.
#[link(wasm_import_module = "objectiveai")]
unsafe extern "C" {
    fn host_execute(argv_ptr: *const u8, argv_len: usize) -> usize;
    fn host_result(out_ptr: *mut u8);
}

/// `Ok(json_array_bytes)` on success, `Err(error_message_bytes)` on host error.
fn call_host(req: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
    let raw = unsafe { host_execute(req.as_ptr(), req.len()) };
    let is_err = raw & ERR_BIT != 0;
    let len = raw & !ERR_BIT;
    let mut buf = vec![0u8; len];
    if len > 0 {
        unsafe { host_result(buf.as_mut_ptr()) };
    }
    if is_err { Err(buf) } else { Ok(buf) }
}

fn main() -> std::process::ExitCode {
    let builder = InterpreterBuilder::new().init_stdlib();
    let def = objectiveai::module_def(&builder.ctx);
    rustpython::run(builder.add_native_module(def))
}

#[rustpython_vm::pymodule]
mod objectiveai {
    use rustpython_vm::{PyObjectRef, PyResult, VirtualMachine};

    /// Run a CLI command in-process on the host and return its streamed output
    /// as a list of native objects (always a list). Raises `RuntimeError` if the
    /// argv fails to parse or the command errors.
    #[pyfunction]
    fn execute(argv: Vec<String>, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let req = serde_json::to_vec(&argv).map_err(|e| {
            vm.new_runtime_error(format!("objectiveai.execute: failed to encode argv: {e}"))
        })?;
        match super::call_host(&req) {
            Ok(bytes) => {
                let json_text = String::from_utf8(bytes).map_err(|e| {
                    vm.new_runtime_error(format!("objectiveai.execute: result was not UTF-8: {e}"))
                })?;
                // Parse via the frozen `json` module so the result is native
                // Python objects (list of dicts/scalars).
                let json = vm.import("json", 0)?;
                json.get_attr("loads", vm)?.call((json_text,), vm)
            }
            Err(bytes) => Err(vm.new_runtime_error(String::from_utf8_lossy(&bytes).into_owned())),
        }
    }
}
