//! Incoming providers: the judges of the ones that dial the daemon.
//!
//! A provider that dials the daemon presents an unbrokered credential,
//! and the daemon judges it by JUDGES the client has added, tried in
//! the order they were added, the first that accepts deciding: a
//! [`Judge::Key`](Judge::Key), a string the credential must equal,
//! naming the identity a peer presenting it has; or a
//! [`Judge::Hook`](Judge::Hook), a directory resource the daemon runs
//! to judge the credential and name the identity, as
//! [`providers`](super) states. None accepting is a closed connection,
//! and nothing of why reaches the peer. The identity a judge names is
//! the provider's, as
//! [`Identity::IncomingUnbrokered`](crate::daemon::endpoints::agents::logs::server::response::Identity::IncomingUnbrokered)
//! carries it.
//!
//! A judge is named afterwards by a [`Reference`]: a key judge by the
//! identity it names, a hook judge by its resource. [`add`] appends
//! one; [`get`] answers one as a list would; [`list`] lists them in the
//! order they are tried, narrowed; [`delete`] takes one out, unless a
//! provider is connected through it; [`edit`] replaces one with another
//! of its kind, keeping its place in the order. What a list and a get
//! report is a [`Told`]: the judge without its key.

mod judge;
mod reference;
mod told;

pub use judge::*;
pub use reference::*;
pub use told::*;

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
