//! The handshake's one frame, written and read.

use bytes::Bytes;
use futures_util::{SinkExt as _, StreamExt as _};

use super::Error;
use crate::shared::discover::Discover;
use crate::wire::connection::Connection;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame::auth::Auth;
use crate::wire::frame::client::ClientFrame;

/// Send `token`, under the discover flag and description `discover`,
/// as a type `0` frame. The frame's layout is the same in both directions, so the
/// client half's encoding serves either end.
pub(super) async fn send(connection: &mut Connection, token: &str, discover: Discover) -> Result<(), Error> {
    let mut payload = Vec::new();
    Auth { discover, token }.encode(&mut Writer::new(&mut payload)).unwrap_or_else(|error| match error {});
    let mut buffer = Vec::new();
    ClientFrame::Auth { payload: &payload }.encode(&mut Writer::new(&mut buffer)).unwrap_or_else(|error| match error {});
    connection.send(Bytes::from(buffer)).await.map_err(|error| Error::Send(error.to_string()))
}

/// The other end's first frame, which must be a type `0` frame with a
/// well-formed payload: its token, and its discover flag and
/// description. The connection ending first is
/// [`Error::Ended`]; any other first frame — a transport error, a frame
/// that will not decode, a frame of another type — is
/// [`Error::NotAuth`]; a malformed payload is [`Error::Malformed`].
pub(super) async fn receive(connection: &mut Connection) -> Result<(String, Discover), Error> {
    let bytes = match connection.next().await {
        None => return Err(Error::Ended),
        Some(Err(_)) => return Err(Error::NotAuth),
        Some(Ok(bytes)) => bytes,
    };
    let Ok(ClientFrame::Auth { payload }) = ClientFrame::decode(&bytes) else {
        return Err(Error::NotAuth);
    };
    let Auth { discover, token } = Auth::decode(payload).map_err(Error::Malformed)?;
    Ok((token.to_string(), discover))
}
