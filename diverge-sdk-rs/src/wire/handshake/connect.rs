//! The connector's half.

use std::net::IpAddr;

use super::frame::{receive, send};
use super::{Authenticator, Error, Handshook};
use crate::shared::listable::Listable;
use crate::wire::connection::Connection;

/// The handshake, from the end that dialled — see the [rules](super):
///
/// 1. This end's token, from `authenticator`, is sent as the
///    connection's first frame, under the listable flag and
///    description `listable` — the caller's, since nothing is known of
///    the connectee yet.
/// 2. The connectee's first frame is read: its token, and its flag and
///    description.
/// 3. That token is verified by `authenticator`, `address` being the
///    connectee's. Its issuer need not be this end's.
///
/// Every failure is the [`Error`], with nothing sent after it; the
/// caller drops the connection, and the close is the whole answer.
pub async fn connect(mut connection: Connection, listable: Listable, authenticator: &dyn Authenticator, address: IpAddr) -> Result<Handshook, Error> {
    let token = authenticator.present().await.map_err(Error::Present)?;
    send(&mut connection, &token, listable).await?;
    let (answer, listable) = receive(&mut connection).await?;
    let peer = authenticator.verify(&answer, address).await.map_err(Error::Refused)?;
    Ok(Handshook { connection, peer, listable })
}
