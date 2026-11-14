//! The handshake's frames, written and read.

use bytes::Bytes;
use futures_util::{SinkExt as _, StreamExt as _};

use super::Error;
use crate::wire::connection::Connection;
use crate::wire::encode::{Encode, Writer};
use crate::wire::frame::client::ClientFrame;

/// Send `payload` as a type `0` frame. The frame's layout is the same
/// in both directions, so the client half's encoding serves either
/// end.
pub(super) async fn send(connection: &mut Connection, payload: &impl Encode<Error = std::convert::Infallible>) -> Result<(), Error> {
    let mut bytes = Vec::new();
    payload.encode(&mut Writer::new(&mut bytes)).unwrap_or_else(|error| match error {});
    let mut buffer = Vec::new();
    ClientFrame::Auth { payload: &bytes }.encode(&mut Writer::new(&mut buffer)).unwrap_or_else(|error| match error {});
    connection.send(Bytes::from(buffer)).await.map_err(|error| Error::Send(error.to_string()))
}

/// The other end's next frame, which must be a type `0` frame: its
/// payload, for the caller to read as the one the handshake expects
/// there. The connection ending first is [`Error::Ended`]; any other
/// frame — a transport error, a frame that will not decode, a frame of
/// another type — is [`Error::NotAuth`].
pub(super) async fn receive(connection: &mut Connection) -> Result<Bytes, Error> {
    let bytes = match connection.next().await {
        None => return Err(Error::Ended),
        Some(Err(_)) => return Err(Error::NotAuth),
        Some(Ok(bytes)) => bytes,
    };
    let Ok(ClientFrame::Auth { payload }) = ClientFrame::decode(&bytes) else {
        return Err(Error::NotAuth);
    };
    Ok(bytes.slice_ref(payload))
}
