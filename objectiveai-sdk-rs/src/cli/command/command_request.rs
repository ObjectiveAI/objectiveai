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
}
