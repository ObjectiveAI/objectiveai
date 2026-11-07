//! A request served on another daemon: this daemon as a conduit.
//!
//! Every request of the wire may carry `relay`, an array of remote
//! daemons' record IDs. [`peel`] finds it in the request's bytes
//! before any handler and takes its first ID off, leaving the rest as
//! the request's `relay`; [`serve`] judges the caller for that first
//! ID alone — the daemons grant's `relay` reaching the record — takes
//! or opens the record's connection, as our account it names, and hands
//! the request to the conduit, [`run`], which puts it on the connection and
//! carries every frame both ways until the remote finishes: the
//! responses, the channels either side opens, their responses and
//! finishes, byte for byte, reading only the headers. The client's
//! scope and channel numbers and the daemon-to-daemon ones are two
//! relationships: each side mints its own, and the conduit keeps the
//! pairs. The remote sees an ordinary request from that account, and
//! nothing of who sent it here; it judges the next ID, if any, as the
//! request's `relay`, and the request itself, as that account's. A relay this daemon will not make is answered as
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
