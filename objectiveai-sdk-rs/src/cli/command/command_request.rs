/// Convert a typed CLI request struct into the argv tail the cli binary
/// should be invoked with. Implementors are the per-leaf request shapes
/// in the surrounding tree (e.g. `agents::spawn::Request`) and emit the
/// flags + positional args their leaf command expects — without the
/// binary name. Callers prepend whatever launcher prefix they need
/// (`["objectiveai-cli"]`, `["objectiveai-cli", "instance"]`, …).
pub trait CommandRequest {
    fn into_command(&self) -> Vec<String>;

    /// Parse an argv tail (the inverse of [`Self::into_command`]) into
    /// the typed request. Default impl returns
    /// [`CommandRequestError::Custom`] until each leaf wires up its
    /// own parser.
    fn from_command(_argv: &[String]) -> Result<Self, CommandRequestError>
    where
        Self: Sized,
    {
        Err(CommandRequestError::Custom(
            "CommandRequest::from_command not yet implemented".to_string(),
        ))
    }
}

/// Error returned by [`CommandRequest::from_command`].
#[derive(Debug)]
pub enum CommandRequestError {
    Serde(serde_path_to_error::Error<serde_json::Error>),
    Custom(String),
}
