/// Convert a typed CLI request struct into the argv tail the cli binary
/// should be invoked with. Implementors are the per-leaf request shapes
/// in the surrounding tree (e.g. `agents::spawn::Request`) and emit the
/// flags + positional args their leaf command expects — without the
/// binary name. Callers prepend whatever launcher prefix they need
/// (`["objectiveai-cli"]`, `["objectiveai-cli", "instance"]`, …).
pub trait CommandRequest {
    fn into_command(&self) -> Vec<String>;

    /// The request's flattened [`RequestBase`] envelope (`jq` /
    /// `python` / `timeout` / `max_tokens`). Every leaf embeds one,
    /// so the implementation is `&self.base`.
    ///
    /// [`RequestBase`]: crate::cli::command::RequestBase
    fn request_base(&self) -> &crate::cli::command::RequestBase;

    /// Mutable access to the request's flattened [`RequestBase`]
    /// envelope, when it has one (`Some(&mut self.base)`). Lets a
    /// caller inject the envelope controls (e.g. `timeout` /
    /// `max_tokens`) onto an already-parsed request in place, without
    /// re-serializing it through argv — used by the MCP server.
    ///
    /// [`RequestBase`]: crate::cli::command::RequestBase
    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase>;
}
