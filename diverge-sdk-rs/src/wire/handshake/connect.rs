//! The connector's half.

use std::net::IpAddr;

use super::frame::{receive, send};
use super::{Authenticator, Error, Handshook};
use crate::shared::listable::Listable;
use crate::wire::connection::Connection;

/// The handshake, from the end that dialled — see the [rules](super):
///
/// 1. This end's token from `issuer`, from `authenticator`, is sent as
///    the connection's first frame, under the listable flag and
///    description `listable` — the caller's, since nothing is known of
///    the connectee yet.
/// 2. The connectee's first frame is read: its token, and its flag and
///    description.
/// 3. That token is verified by `authenticator`, `address` being the
///    connectee's, and its issuer must be `issuer`.
///
/// An answer step 3 accepted but whose issuer is not `issuer` is
/// reported to `authenticator` as
/// [`abandon`](Authenticator::abandon)ed. Every failure is the
/// [`Error`], with nothing sent after it; the
/// caller drops the connection, and the close is the whole answer.
pub async fn connect(
    mut connection: Connection,
    issuer: &str,
    listable: Listable,
    authenticator: &dyn Authenticator,
    address: IpAddr,
) -> Result<Handshook, Error> {
    let token = authenticator.present(issuer).await.map_err(Error::Present)?;
    send(&mut connection, &token, listable).await?;
    let (answer, listable) = receive(&mut connection).await?;
    let peer = authenticator.verify(&answer, address).await.map_err(Error::Refused)?;
    if peer.issuer != issuer {
        authenticator.abandon(&peer).await;
        return Err(Error::Issuer { sent: issuer.to_string(), answered: peer.issuer });
    }
    Ok(Handshook { connection, peer, listable })
}
