//! Opening the scope.

use super::super::request;
use super::{Cancel, ExecuteError, Sending};
use crate::wire::client::handle::Handle;
use crate::wire::encode::{Encode, Writer};

/// Send the message, and hand back what waits for its fate:
/// [`Sending::wait`] reads the one answer — see
/// [`response::Frame`](crate::provider::endpoints::daemons::message::server::response::Frame)
/// — or the finish with nothing before it, and [`Sending::cancel`]
/// takes the message back while it waits.
pub async fn execute(handle: &Handle, request: &request::Frame<'_>) -> Result<Sending, ExecuteError> {
    let mut payload = Vec::new();
    request.encode(&mut Writer::new(&mut payload)).map_err(ExecuteError::Request)?;
    let scope = handle.send_request(&payload).await.map_err(ExecuteError::Send)?;
    let cancel = Cancel::new(handle.clone(), scope.scope);
    Ok(Sending::new(cancel, scope.response_receiver))
}
