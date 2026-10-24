//! The handshake's one frame, written and read.

use bytes::Bytes;
use futures_util::{SinkExt as _, StreamExt as _};

use super::Error;
use crate::wire::connection::Connection;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame::auth::Auth;
use crate::wire::frame::client::ClientFrame;

/// Send `token` as a type `0` frame. The frame's layout is the same in
/// both directions, so the client half's encoding serves either end.
pub(super) async fn send(connection: &mut Connection, token: &str) -> Result<(), Error> {
    let mut payload = Vec::new();
    Auth(token).encode(&mut Writer::new(&mut payload)).unwrap_or_else(|error| match error {});
    let mut buffer = Vec::new();
    ClientFrame::Auth { payload: &payload }.encode(&mut Writer::new(&mut buffer)).unwrap_or_else(|error| match error {});
    connection.send(Bytes::from(buffer)).await.map_err(|error| Error::Send(error.to_string()))
}

/// The other end's first frame, which must be a type `0` frame with a
/// well-formed token: the token. The connection ending first is
/// [`Error::Ended`]; any other first frame — a transport error, a frame
/// that will not decode, a frame of another type — is
/// [`Error::NotAuth`]; a malformed token is [`Error::Malformed`].
pub(super) async fn receive(connection: &mut Connection) -> Result<String, Error> {
    let bytes = match connection.next().await {
        None => return Err(Error::Ended),
        Some(Err(_)) => return Err(Error::NotAuth),
        Some(Ok(bytes)) => bytes,
    };
    let Ok(ClientFrame::Auth { payload }) = ClientFrame::decode(&bytes) else {
        return Err(Error::NotAuth);
    };
    let Auth(token) = Auth::decode(payload).map_err(Error::Malformed)?;
    Ok(token.to_string())
}
