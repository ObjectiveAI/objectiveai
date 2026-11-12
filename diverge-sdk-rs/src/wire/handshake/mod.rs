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
//!    directions. Its payload is a token and nothing else: an RFC 7519
//!    JWT as UTF-8, every byte of it — see
//!    [`Auth`](crate::wire::frame::auth::Auth). An empty payload, or a
//!    payload that is not UTF-8, is malformed.
//! 2. **Each end sends its token.** The first frame an end sends is
//!    its token — the one identity it presents itself as, whatever
//!    issuer the other end's token is from. The two tokens need not
//!    share an issuer.
//! 3. **Both ends send.** The connector sends one and the connectee
//!    sends one. The steps [`connect`] and [`accept`] take are each
//!    function's own, and no rule of the protocol.
//! 4. **Nothing else without acceptance.** An end that has not
//!    verified the other's token sends nothing but its own, and
//!    serves nothing.
//! 5. **Judgment is each end's own.** Which token an end presents,
//!    which issuers it trusts, the signature, the expiry, the
//!    audience, and whether the token's `(iss, sub)` is one it admits
//!    are the [`Authenticator`]'s, not the protocol's.
//! 6. **Failure is a close.** A first frame that is not type `0`, a
//!    malformed token, a token an end does not accept, and an end with
//!    no token to present each end the connection, and no frame says
//!    why — in either direction.
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
