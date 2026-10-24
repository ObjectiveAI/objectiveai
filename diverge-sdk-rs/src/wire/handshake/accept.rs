//! The connectee's half.

use std::net::IpAddr;

use super::frame::{receive, send};
use super::{Authenticator, Error, Handshook};
use crate::wire::connection::Connection;

/// The handshake, from the end that accepted — see the [rules](super):
///
/// 1. The connector's first frame is read: its token.
/// 2. That token is verified by `authenticator`, `address` being the
///    connector's, which names the connector's issuer.
/// 3. This end's token from that same issuer, from `authenticator`, is
///    sent as this end's first frame.
///
/// A failure after step 2 accepted the connector is reported to
/// `authenticator` as [`abandon`](Authenticator::abandon)ed. Every
/// failure is the [`Error`], with nothing sent: a connector this
/// end does not accept, or one whose issuer this end holds no identity
/// at, earns no frame. The caller drops the connection, and the close
/// is the whole answer.
pub async fn accept(mut connection: Connection, authenticator: &dyn Authenticator, address: IpAddr) -> Result<Handshook, Error> {
    let token = receive(&mut connection).await?;
    let peer = authenticator.verify(&token, address).await.map_err(Error::Refused)?;
    let answered = match authenticator.present(&peer.issuer).await {
        Ok(answer) => send(&mut connection, &answer).await,
        Err(reason) => Err(Error::Present(reason)),
    };
    if let Err(error) = answered {
        authenticator.abandon(&peer).await;
        return Err(error);
    }
    Ok(Handshook { connection, peer })
}
