//! The handshake: each end's token and its proof of the key the token
//! is bound to, before the connection is anything else — and the keys every
//! frame after it is sealed under.
//!
//! A connection's **connector** is the end that dialled; its
//! **connectee**, the end that accepted. Either may be the client half
//! or the server half — a daemon dials a provider, and a provider dials
//! a daemon — so the handshake belongs to the
//! [`Connection`](crate::wire::connection::Connection), before the
//! client's [`Router`](crate::wire::client::router::Router) or the
//! server's [`Session`](crate::wire::server::session::Session) is built
//! on it. The rules:
//!
//! 1. **The frame.** Type `0`, scope and channel `0`. There are three,
//!    each its own payload — see
//!    [`auth`](crate::wire::frame::auth): the connector's
//!    [`Offer`](crate::wire::frame::auth::Offer), the connectee's
//!    [`Answer`](crate::wire::frame::auth::Answer), and the connector's
//!    [`Proof`](crate::wire::frame::auth::Proof), in that order.
//! 2. **A token is bound to a key.** Each end's token is an access
//!    token of the one identity it presents itself as, bound to a
//!    P-256 public key as RFC 9449 binds one: its `cnf` claim carries
//!    `jkt`, the key's RFC 7638 thumbprint — see [`thumbprint`]. The
//!    two tokens need not share an issuer.
//! 3. **The offer.** The connector makes a one-time P-256 key pair
//!    and sends its public half with its identity's public key and
//!    its token. It is the first frame of the connection.
//! 4. **The answer.** The connectee judges the connector's token, and
//!    holds the connector's key to the thumbprint the token names.
//!    Both good, it answers: a one-time key pair of its own, whose
//!    public half it sends with its identity's public key, its token,
//!    and its signature over the [transcript](#the-transcript) by its
//!    identity key. Either bad, it closes, sending nothing.
//! 5. **The proof.** The connector judges the connectee's token,
//!    holds the connectee's key to the thumbprint that token names,
//!    and checks the signature against the key. All good, it sends
//!    its own signature over the transcript, by its own identity key;
//!    the connectee checks it against the connector's key.
//! 6. **Nothing else without acceptance.** An end sends no frame but
//!    these, and serves none, until the handshake is whole.
//! 7. **Judgment is each end's own.** Which token an end presents,
//!    which issuers it trusts, the token's signature, its expiry, and
//!    whether its `(iss, sub)` is one it admits are the
//!    [`Authenticator`]'s, not the protocol's. That the token is
//!    bound to a key, that the key sent is that key, and that the
//!    other end proves it holds it, are the protocol's.
//! 8. **Failure is a close.** A first frame that is not type `0`, a
//!    malformed payload, a token an end does not accept, a key that
//!    is not the token's, a signature that does not check, a key
//!    exchange that yields nothing, and an end with no token to
//!    present each end the connection, and no frame says why — in
//!    either direction.
//! 9. **Exactly these.** A type `0` frame after the handshake, in
//!    either direction, ends the connection.
//! 10. **What it establishes.** Each end holds the other's [`Peer`] —
//!     the pair `(iss, sub)` — for the connection's life, and the two
//!     session keys: every message after the proof is one frame
//!     sealed, see [`Seal`](crate::wire::seal::Seal).
//!
//! # The transcript
//!
//! What both signatures cover, and the session keys are drawn under:
//!
//! ```text
//! "diverge handshake"            17 bytes, ASCII
//! the connector's one-time key   65 bytes
//! the connectee's one-time key   65 bytes
//! the connector's token's length u32, big-endian
//! the connector's token          its bytes
//! the connectee's token's length u32, big-endian
//! the connectee's token          its bytes
//! ```
//!
//! 1. **What is signed.** The transcript, then one byte for who
//!    signs: `0` the connector, `1` the connectee. So neither end's
//!    signature is the other's, and a signature made for one
//!    connection is good for no other: the one-time keys are its own.
//!    The identity keys are not in it: each token names its own, by
//!    thumbprint, and the tokens are.
//! 2. **The signature.** ECDSA over P-256 with SHA-256, FIPS 186-4,
//!    as the 32 bytes of `r` then the 32 bytes of `s`. Either value
//!    of `s` a signature may carry is accepted.
//! 3. **The shared secret.** Elliptic-curve Diffie-Hellman over
//!    P-256, SEC 1 §3.3.1, of an end's one-time private key and the
//!    other's one-time public key: the 32 bytes of the shared point's
//!    `x` coordinate. A one-time key that is no point of the curve
//!    ends the handshake.
//! 4. **The session keys.** HKDF-SHA-256, RFC 5869: the salt is the
//!    SHA-256 of the transcript, the input key the shared secret, and
//!    the output thirty-two bytes under each of two infos: the ASCII
//!    `diverge connector`, the key of what the connector sends, and
//!    `diverge connectee`, the key of what the connectee sends.
//! 5. **The one-time keys are let go** once the session keys are
//!    drawn: what was said on a connection stays unread by whoever
//!    later learns an identity's key.
//!
//! [`connect`] is the connector's half and [`accept`] the connectee's;
//! each hands the connection back, sealed, as a [`Handshook`], once
//! its half is done.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod accept;
mod authenticator;
mod connect;
mod error;
mod exchange;
mod frame;
mod handshook;
mod peer;
mod thumbprint;
mod transcript;

pub use accept::*;
pub use authenticator::*;
pub use connect::*;
pub use error::*;
pub use handshook::*;
pub use peer::*;
pub use thumbprint::*;
