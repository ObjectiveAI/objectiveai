//! Running a command a container asked for.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

use crate::shared::error::Error;

/// The caller's side of [`command`](crate::shared::containers::command):
/// a container has no CLI and no daemon it may dial, so a command it
/// wants run is run by whoever can, and that is the caller.
///
/// The command is bytes in the CLI's vocabulary, which this crate
/// never reads; the answer is one item per frame, likewise opaque,
/// then the finish — or an [`Error`], last, which ends the exchange.
/// What a container may ask for is settled between it and the
/// caller.
pub trait CommandRunner: Send + Sync {
    /// Run one command: its items in order, or the error that ends
    /// them.
    fn run(
        &self,
        command: Bytes,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + 'static>>> + Send;
}
