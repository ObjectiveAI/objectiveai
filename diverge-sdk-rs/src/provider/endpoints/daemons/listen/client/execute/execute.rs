//! Opening the scope.

use super::super::request;
use super::{Cancel, ExecuteError, ExecuteStream};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Ask the provider for every message sent to the caller's identity,
/// and keep asking.
///
/// Reads nothing before returning: the provider's first frame is the
/// first message or a refusal, and both are the stream's. The stream
/// ends at the provider's finish, which follows the [`Cancel`] sent or
/// the connection ending. Every message the stream yields is taken
/// when it is yielded: the provider answers its sender delivered once
/// the stream has handed it over, so a caller reads a message only
/// when it is ready for it.
pub async fn execute(handle: &Handle, request: &request::Frame) -> Result<(ExecuteStream, Cancel), ExecuteError> {
    let mut payload = Vec::new();
    // Taken as an argument like every other listing's, though it has
    // no fields to carry. Its encode is `Infallible`.
    request.encode(&mut Writer::new(&mut payload)).unwrap_or_else(|error| match error {});
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let cancel = Cancel::new(handle.clone(), scope.scope);
    Ok((ExecuteStream::new(scope.response_receiver), cancel))
}
