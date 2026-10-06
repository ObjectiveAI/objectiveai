//! Providers: the two ways the daemon comes to be connected to one.
//!
//! A provider is where containers run and volumes live, and the daemon
//! reaches it one of two ways, which its
//! [`Identity`](crate::daemon::endpoints::agents::logs::server::response::Identity)
//! already tells apart. An [`outgoing`] provider the daemon DIALS: a
//! client gives an address and a mode, by the mode's name as the
//! provider server's own `auth` names its modes — `unbrokered`, with
//! the credential the daemon presents as its own — and the daemon dials
//! it for whatever it needs of it, presenting that credential first, as
//! the wire rules. An [`incoming`] provider dials the daemon, and the
//! daemon JUDGES its credential as a provider judges its own peers: by
//! the credentials the client has added, each a key the daemon minted
//! naming the identity a peer presenting it has; the key that equals
//! the presented one decides, and none is a closed connection.
//!
//! # Two secrets, two directions
//!
//! An outgoing provider's authorization is the provider's: the client
//! gives it, a list and a get report the mode without it, and an edit
//! replaces it whole, which is how one rotates. An incoming
//! credential's key is the daemon's: the client never gives one — the
//! daemon mints it when the credential is added or replaced, answers it
//! in that response, once, and reports it never.

pub mod incoming;
pub mod outgoing;
