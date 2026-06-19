/// Common accessors shared by the per-leaf typed CLI request shapes in the
/// surrounding tree (e.g. `agents::spawn::Request`): the flattened
/// [`RequestBase`] envelope every leaf embeds.
///
/// Lowering a request to an argv tail no longer lives here — requests are
/// dispatched to the cli as JSON via its top-level `--request` flag, so the
/// SDK serializes the typed request directly rather than building argv.
///
/// [`RequestBase`]: crate::cli::command::RequestBase
pub trait CommandRequest {
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
