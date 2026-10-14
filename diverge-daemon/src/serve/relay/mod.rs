//! A request served on another daemon: this daemon as a conduit.
//!
//! Every request of the wire may carry `relay`, naming a daemon
//! record and one of its accounts. [`peel`] finds it in the request's
//! bytes before any handler and takes it out; [`serve`] judges the
//! caller — the daemons grant's `relay` action over the record, the
//! account among the grant's `accounts` — takes or opens the
//! connection to that daemon as that account, and hands the stripped
//! request to the conduit, [`run`], which puts it on the connection and
//! carries every frame both ways until the remote finishes: the
//! responses, the channels either side opens, their responses and
//! finishes, byte for byte, reading only the headers. The client's
//! scope and channel numbers and the daemon-to-daemon ones are two
//! relationships: each side mints its own, and the conduit keeps the
//! pairs. The remote sees an ordinary request from that account, and
//! nothing of who sent it here; with the member taken out, nothing is
//! relayed twice. A relay this daemon will not make is answered as
//! the request's own `forbidden` or `error`, which every endpoint
//! has, so a client reading the endpoint's frames reads it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod conduit;
mod peel;
mod serve;

pub use conduit::*;
pub use peel::*;
pub use serve::*;
