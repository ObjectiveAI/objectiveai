/// Convert a typed CLI request struct into the argv tail the cli binary
/// should be invoked with. Implementors are the per-leaf request shapes
/// in the surrounding tree (e.g. `agents::spawn::Request`) and emit the
/// flags + positional args their leaf command expects — without the
/// binary name. Callers prepend whatever launcher prefix they need
/// (`["objectiveai-cli"]`, `["objectiveai-cli", "instance"]`, …).
pub trait IntoCommand {
    fn into_command(&self) -> Vec<String>;
}
