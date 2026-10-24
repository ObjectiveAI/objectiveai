//! The handshake: each end's token, before the connection is anything
//! else.
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
//! 1. **The frame.** Type `0`, scope and channel `0`, one layout in both
//!    directions. Its payload is a token — an RFC 7519 JWT, as UTF-8,
//!    the whole payload — see [`Auth`](crate::wire::frame::auth::Auth).
//!    An empty payload, or one that is not UTF-8, is malformed.
//! 2. **The connector speaks first.** Its first frame is its token from
//!    the issuer of its choice, and it sends nothing else until the
//!    connectee has answered.
//! 3. **The connectee answers.** It reads the connector's token,
//!    verifies it, and sends its own token as its first frame — one
//!    from the SAME issuer: the `iss` of the token it received,
//!    exactly. It serves nothing before it has answered.
//! 4. **The connector checks the answer.** It verifies the connectee's
//!    token, and its `iss` must be the issuer it presented.
//! 5. **Judgment is each end's own.** Which issuers an end trusts, the
//!    signature, the expiry, the audience, and whether the token's
//!    `(iss, sub)` is one it admits are the [`Authenticator`]'s, not
//!    the protocol's.
//! 6. **Failure is a close.** A first frame that is not type `0`, a
//!    malformed token, a token an end does not accept, a connectee with
//!    no token from the connector's issuer, and an answer from another
//!    issuer each end the connection, and no frame says why — in either
//!    direction.
//! 7. **Exactly one each.** A type `0` frame after the handshake, in
//!    either direction, ends the connection.
//! 8. **What it establishes.** Each end holds the other's [`Peer`] —
//!    the pair `(iss, sub)` — for the connection's life.
//!
//! [`connect`] is the connector's half and [`accept`] the connectee's;
//! each hands the connection back, as a [`Handshook`], once its half is
//! done.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod accept;
mod authenticator;
mod connect;
mod error;
mod frame;
mod handshook;
mod peer;

pub use accept::*;
pub use authenticator::*;
pub use connect::*;
pub use error::*;
pub use handshook::*;
pub use peer::*;
