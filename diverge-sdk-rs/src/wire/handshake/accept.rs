//! The connectee's half.

use std::net::IpAddr;

use super::exchange::Exchange;
use super::frame::{receive, send};
use super::transcript::{Role, Transcript};
use super::{Authenticator, Error, Handshook};
use crate::wire::connection::Connection;
use crate::wire::decode::Decode as _;
use crate::wire::frame::auth::{Answer, Offer, Proof};

/// The handshake, from the end that accepted — see the [rules](super):
///
/// 1. The connector's first frame is read: its one-time key and its
///    token.
/// 2. That token is verified by `authenticator`, `address` being the
///    connector's.
/// 3. This end's token and signer, from `authenticator` — the one
///    identity it answers as, whatever the connector's issuer — and a
///    one-time key pair made for this connection. The pair's public
///    half, this end's signature over the transcript, and the token
///    are sent as this end's one handshake frame.
/// 4. The connector's last handshake frame is read: its signature,
///    checked against the key its token names, over the transcript.
/// 5. The shared secret is agreed. One that is all zero ends the
///    handshake.
/// 6. The connection is sealed: what this end sends under the
///    connectee's key, what it receives under the connector's.
///
/// A failure after step 2 accepted the connector is reported to
/// `authenticator` as [`abandon`](Authenticator::abandon)ed. Every
/// failure is the [`Error`], with nothing sent after it: a connector
/// this end does not accept earns no frame, and neither does any
/// connector of an end that has no identity to answer as. The caller
/// drops the connection, and the close is the whole answer.
pub async fn accept(mut connection: Connection, authenticator: &dyn Authenticator, address: IpAddr) -> Result<Handshook, Error> {
    // Step 1.
    let payload = receive(&mut connection).await?;
    let offer = Offer::decode(&payload).map_err(Error::Malformed)?;
    // Step 2.
    let judged = authenticator.verify(offer.token, address).await.map_err(Error::Refused)?;
    // Steps 3 to 5: any failure lets go of what step 2 took.
    let sealed = async {
        let presented = authenticator.present().await.map_err(Error::Present)?;
        let exchange = Exchange::new();
        let ours = exchange.public();
        let transcript = Transcript::new(&offer.key, &ours, offer.token, &presented.token);
        let signature = presented.signer.sign(&transcript.signed(Role::Connectee));
        send(&mut connection, &Answer { key: ours, signature, token: &presented.token }).await?;
        let payload = receive(&mut connection).await?;
        let proof = Proof::decode(&payload).map_err(Error::Malformed)?;
        if !transcript.proven(Role::Connector, &judged.key, &proof.signature) {
            return Err(Error::Signature);
        }
        let shared = exchange.agree(&offer.key).ok_or(Error::Exchange)?;
        Ok(transcript.seals(&shared))
    }
    .await;
    match sealed {
        // Step 6.
        Ok((connector, connectee)) => Ok(Handshook { connection: connection.sealed(connectee, connector), peer: judged.peer }),
        Err(error) => {
            authenticator.abandon(&judged.peer).await;
            Err(error)
        }
    }
}
