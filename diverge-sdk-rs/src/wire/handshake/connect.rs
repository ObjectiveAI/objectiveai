//! The connector's half.

use std::net::IpAddr;

use super::exchange::Exchange;
use super::frame::{receive, send};
use super::transcript::{Role, Transcript};
use super::{Authenticator, Error, Handshook, thumbprint};
use crate::wire::connection::Connection;
use crate::wire::decode::Decode as _;
use crate::wire::frame::auth::{Answer, Offer, Proof};

/// The handshake, from the end that dialled — see the [rules](super):
///
/// 1. This end's token, its key and its signer, from `authenticator`,
///    and a one-time key pair made for this connection. The pair's
///    public half, the identity key and the token are sent as the
///    connection's first frame.
/// 2. The connectee's frame is read: its one-time key, its identity
///    key, its signature, its token.
/// 3. That token is verified by `authenticator`, `address` being the
///    connectee's. Its issuer need not be this end's.
/// 4. The connectee's identity key is the one its token names: its
///    thumbprint is the token's.
/// 5. The connectee's signature is checked against that key, over the
///    transcript.
/// 6. The shared secret is agreed. A one-time key that is no point of
///    the curve ends the handshake.
/// 7. This end's signature over the transcript is sent as its last
///    handshake frame.
/// 8. The connection is sealed: what this end sends under the
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
    send(&mut connection, &Offer { exchange: ours, key: presented.key, token: &presented.token }).await?;
    // Step 2.
    let payload = receive(&mut connection).await?;
    let answer = Answer::decode(&payload).map_err(Error::Malformed)?;
    // Step 3.
    let judged = authenticator.verify(answer.token, address).await.map_err(Error::Refused)?;
    let transcript = Transcript::new(&ours, &answer.exchange, &presented.token, answer.token);
    // Steps 4 to 7: any failure lets go of what step 3 took.
    let proven = async {
        if thumbprint(&answer.key) != judged.thumbprint {
            return Err(Error::Key);
        }
        if !transcript.proven(Role::Connectee, &answer.key, &answer.signature) {
            return Err(Error::Signature);
        }
        let shared = exchange.agree(&answer.exchange).ok_or(Error::Exchange)?;
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
    // Step 8.
    let (connector, connectee) = transcript.seals(&shared);
    Ok(Handshook { connection: connection.sealed(connector, connectee), peer: judged.peer })
}
