use futures::Stream;

use crate::cli::command::CommandRequest;

/// Run a [`CommandRequest`] against some backend (subprocess of the cli
/// binary, in-process router, mock, …) and surface its output as a
/// stream of typed items.
///
/// `T` is left to the caller: pick a concrete leaf response type
/// (`agents::spawn::Response`, `functions::executions::create::standard::ResponseItem`,
/// …) or a more general `serde_json::Value` for opaque consumption.
pub trait CommandExecutor {
    type Error;
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
}
