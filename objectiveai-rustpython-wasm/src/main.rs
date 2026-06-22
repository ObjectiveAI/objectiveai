//! Custom WASI RustPython interpreter with the `objectiveai` native module.
//!
//! This is the stock `rustpython` 0.5.0 binary (argv parsing, `-c <code>`,
//! `_start`, exit codes, frozen stdlib via `init_stdlib`) plus one native
//! module — `objectiveai` — exposing:
//!
//!   objectiveai.execute(argv: list[str])        -> list           # one command
//!   objectiveai.execute(argvs: list[list[str]]) -> list[list]     # parallel batch
//!
//! `execute` runs CLI command(s) **in-process on the host**, with the output
//! shape mirroring the input: a single argv returns that command's streamed
//! output as a list of native Python objects; a batch returns a list of those
//! lists, running the argvs in parallel. It is implemented by calling a wasm
//! import `host_execute`/`host_result` that the wasmtime host (the embedder,
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

    /// Run CLI command(s) in-process on the host. Polymorphic, with the output
    /// shape mirroring the input:
    ///   - a single argv (`list[str]`)  → a list of native objects (the items);
    ///   - a batch (`list[list[str]]`) → a list of those lists, the argvs run
    ///     IN PARALLEL.
    /// An empty list is an empty batch (returns `[]`). Raises `RuntimeError` on a
    /// parse or command error (a batch is all-or-nothing). Recursion is allowed
    /// (a command run here may itself run Python that calls `execute` again).
    #[pyfunction]
    fn execute(request: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        // Serialize the request (list[str] OR list[list[str]]) to JSON via the
        // frozen `json` module, so one fn handles both shapes; the host detects
        // which and returns the matching shape. Parse the reply with `json` too,
        // so it comes back as native objects.
        let json = vm.import("json", 0)?;
        let request: String = json
            .get_attr("dumps", vm)?
            .call((request,), vm)?
            .try_into_value(vm)?;
        match super::call_host(request.as_bytes()) {
            Ok(bytes) => {
                let resp = String::from_utf8(bytes).map_err(|e| {
                    vm.new_runtime_error(format!("objectiveai.execute: result was not UTF-8: {e}"))
                })?;
                json.get_attr("loads", vm)?.call((resp,), vm)
            }
            Err(bytes) => Err(vm.new_runtime_error(String::from_utf8_lossy(&bytes).into_owned())),
        }
    }
}
