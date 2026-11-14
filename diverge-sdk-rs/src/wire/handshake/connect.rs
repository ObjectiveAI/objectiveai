//! The connector's half.

use std::net::IpAddr;

use super::exchange::Exchange;
use super::frame::{receive, send};
use super::transcript::{Role, Transcript};
use super::{Authenticator, Error, Handshook};
use crate::wire::connection::Connection;
use crate::wire::decode::Decode as _;
use crate::wire::frame::auth::{Answer, Offer, Proof};

/// The handshake, from the end that dialled — see the [rules](super):
///
/// 1. This end's token and signer, from `authenticator`, and a
///    one-time key pair made for this connection. The pair's public
///    half and the token are sent as the connection's first frame.
/// 2. The connectee's frame is read: its one-time key, its signature,
///    its token.
/// 3. That token is verified by `authenticator`, `address` being the
///    connectee's. Its issuer need not be this end's.
/// 4. The connectee's signature is checked against the key its token
///    names, over the transcript.
/// 5. The shared secret is agreed. One that is all zero ends the
///    handshake.
/// 6. This end's signature over the transcript is sent as its last
///    handshake frame.
/// 7. The connection is sealed: what this end sends under the
///    connector's key, what it receives under the connectee's.
///
/// A failure after step 3 accepted the connectee is reported to
/// `authenticator` as [`abandon`](Authenticator::abandon)ed. Every
/// failure is the [`Error`], with nothing sent after it; the caller
/// drops the connection, and the close is the whole answer.
pub async fn connect(mut connection: Connection, authenticator: &dyn Authenticator, address: IpAddr) -> Result<Handshook, Error> {
    // Step 1.
    let presented = authenticator.present().await.map_err(Error::Present)?;
    let exchange = Exchange::new();
    let ours = exchange.public();
    send(&mut connection, &Offer { key: ours, token: &presented.token }).await?;
    // Step 2.
    let payload = receive(&mut connection).await?;
    let answer = Answer::decode(&payload).map_err(Error::Malformed)?;
    // Step 3.
    let judged = authenticator.verify(answer.token, address).await.map_err(Error::Refused)?;
    let transcript = Transcript::new(&ours, &answer.key, &presented.token, answer.token);
    // Steps 4 to 6: any failure lets go of what step 3 took.
    let proven = async {
        if !transcript.proven(Role::Connectee, &judged.key, &answer.signature) {
            return Err(Error::Signature);
        }
        let shared = exchange.agree(&answer.key).ok_or(Error::Exchange)?;
        let signature = presented.signer.sign(&transcript.signed(Role::Connector));
        send(&mut connection, &Proof { signature }).await?;
        Ok(shared)
    }
    .await;
    let shared = match proven {
        Ok(shared) => shared,
        Err(error) => {
            authenticator.abandon(&judged.peer).await;
            return Err(error);
        }
    };
    // Step 7.
    let (connector, connectee) = transcript.seals(&shared);
    Ok(Handshook { connection: connection.sealed(connector, connectee), peer: judged.peer })
}
