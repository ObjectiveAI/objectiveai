//! The Diverge daemon: the server of the daemon protocol.
//!
//! A daemon is what a client of the daemon protocol —
//! [`diverge_sdk::daemon`] — asks for agents, tools, resources,
//! providers, accounts, roles, volumes and the database. The SDK
//! defines every request and every answer, and the frame-level
//! server, [`diverge_sdk::wire::server`], that reads scopes off a
//! socket; what it does not hold is the daemon itself — the judgment
//! of who is asking, the records of what exists, and the doing — and
//! this crate is where that goes.
//!
//! # Initialized, not yet serving
//!
//! The crate as it stands is the shape of the daemon and none of its
//! substance: it finds its directory, reads its file, binds its port,
//! accepts every WebSocket, reads the first frame of each as a
//! credential, and reads every request as the [`ClientRequest`] it
//! is. Then it does the one thing it can do without records: it
//! answers every request with that endpoint's own error, saying so,
//! and finishes the scope. No credential is admitted yet either,
//! since there is no account to admit it as — the root key the first
//! client will present, and the accounts after it, come with the
//! store that will hold them, which is not decided here. What this
//! buys is that every piece that follows has a place: the port, the
//! session, the dispatch over all eighty-nine requests, and the
//! refusal every one of them gives today.
//!
//! # The directory
//!
//! `--config <dir>`, else `DIVERGE_DAEMON_CONFIG`, else
//! `~/.diverge/daemon/`, holding `config.yaml`, optional.
//!
//! [`config`] is what the daemon is told; [`serve`] is the daemon
//! running: the port bound, every connection read, every request
//! answered, and the stop.
//!
//! [`ClientRequest`]: diverge_sdk::daemon::endpoints::ClientRequest

pub mod config;
pub mod serve;
