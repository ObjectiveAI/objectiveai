use futures::Stream;

use crate::cli::command::CommandRequest;
use crate::cli::command::CommandResponse;

pub mod binary;
pub mod plugin;


pub use binary::BinaryExecutor;


/// Run a [`CommandRequest`] against some backend (subprocess of the cli
/// binary, in-process router, mock, …) and surface its output as a
/// stream of typed items.
///
/// `T` is left to the caller: pick a concrete leaf response type
/// (`agents::spawn::Response`, `functions::execute::standard::ResponseItem`,
/// …) or a more general `serde_json::Value` for opaque consumption.
///
/// Every call accepts an optional [`Identity`] bag controlling
/// per-call identity. When `Some`, subprocess-spawning executors stamp
/// each `Some(v)` field on the child env and `env_remove` each `None`
/// — atomic per-call override. When `None`, the child inherits parent
/// env unchanged. In-process executors (e.g. the plugin executor)
/// accept the parameter for trait-signature symmetry and ignore it.
pub trait CommandExecutor {
    type Error: Send + 'static;
    type Stream<T>: Stream<Item = Result<T, Self::Error>> + Send + 'static
    where
        T: Send + 'static;

    fn execute<R, T>(
        &self,
        request: R,
        identity: Option<&crate::identity::Identity>,
    ) -> impl Future<Output = Result<Self::Stream<T>, Self::Error>> + Send
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static;

    /// Convenience for unary commands: run the request and resolve the
    /// first item from the stream. Implementations should error with
    /// their own "empty stream" variant if the stream closes without
    /// producing an item.
    fn execute_one<R, T>(
        &self,
        request: R,
        identity: Option<&crate::identity::Identity>,
    ) -> impl Future<Output = Result<T, Self::Error>> + Send
    where
        R: CommandRequest + Send + serde::Serialize,
        T: CommandResponse + serde::Serialize + serde::de::DeserializeOwned + Send + 'static;
}
