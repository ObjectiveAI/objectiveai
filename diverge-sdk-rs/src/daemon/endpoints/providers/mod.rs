//! Providers: the two ways the daemon comes to be connected to one,
//! and the daemons it reaches through them.
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
//! No request and no answer carries a key; each names an entry in
//! the daemon's host's [`keyring`](crate::keyring). An outgoing
//! provider's key is the provider's: the client puts it in the keyring
//! and names the entry in the mode, a list and a get report the mode
//! without it, and an edit replaces the mode whole. An incoming
//! credential's key is the daemon's: the daemon mints it when the
//! credential is added or replaced, keeps it in the keyring, and
//! answers the entry's name.
//!
//! # Where the daemon stands with each
//!
//! A list or a get of either reports the daemon's [`Status`] with
//! the provider: no connection; one the daemon uses, with the
//! version and the capabilities the provider answered its `info`
//! ask, cached for the connection's life; or one the daemon holds
//! open and never uses, the provider's version not being the
//! daemon's. The daemon asks a provider nothing its capabilities
//! exclude, and counts it connected only once the answer is cached.
//! Beside the status, an item carries the provider's latest
//! `capacity` answer, which the daemon polls for and caches; a list
//! kept open hears each new answer as the item changed.
//!
//! # And the daemons beyond them
//!
//! A [`daemons`] record is another daemon the caller holds an account
//! on, reached through a provider both are connected to and never
//! otherwise: its name here, the mode this daemon authenticates to it
//! in — an outgoing provider's modes, the credential one the remote
//! holds as an incoming credential — and one link per provider it is
//! reachable through, each a provider of the caller's and the identity
//! the remote is known by there. A connected tool names one.

mod status;

pub use status::*;

pub mod daemons;
pub mod incoming;
pub mod outgoing;
