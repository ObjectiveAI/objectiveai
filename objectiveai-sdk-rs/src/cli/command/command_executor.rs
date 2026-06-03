use futures::Stream;

use crate::cli::command::CommandRequest;

pub mod binary;
pub mod plugin;

/// Run a [`CommandRequest`] against some backend (subprocess of the cli
/// binary, in-process router, mock, …) and surface its output as a
/// stream of typed items.
///
/// `T` is left to the caller: pick a concrete leaf response type
/// (`agents::spawn::Response`, `functions::executions::create::standard::ResponseItem`,
/// …) or a more general `serde_json::Value` for opaque consumption.
pub trait CommandExecutor {
    type Error: Send + 'static;
    type Stream<T>: Stream<Item = Result<T, Self::Error>> + Send + 'static
    where
        T: Send + 'static;

    fn execute<R, T>(
        &self,
        request: R,
    ) -> impl Future<Output = Result<Self::Stream<T>, Self::Error>> + Send
    where
        R: CommandRequest + Send,
        T: serde::de::DeserializeOwned + Send + 'static;

    /// Convenience for unary commands: run the request and resolve the
    /// first item from the stream. Implementations should error with
    /// their own "empty stream" variant if the stream closes without
    /// producing an item.
    fn execute_one<R, T>(
        &self,
        request: R,
    ) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        R: CommandRequest + Send,
        T: serde::de::DeserializeOwned + Send + 'static;
}
