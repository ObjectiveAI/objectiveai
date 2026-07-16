//! Consumer + wire types for the cli daemon's `/laboratories/list`
//! endpoint — the `laboratories list` merge as a live stream.
//!
//! On connect the daemon sends one [`LaboratoryEvent::Snapshot`] with
//! every known laboratory — the union of the daemon's live
//! `/laboratory` connections and the machine's local container scan,
//! each classified `source` local/remote by RAW id (the same rules as
//! the unary `laboratories list` command) plus a live `connected`
//! flag — then streams [`LaboratoryEvent::Upserted`] /
//! [`LaboratoryEvent::Removed`] deltas as laboratories connect,
//! disconnect, appear in, or vanish from the local scan.
//!
//! The daemon side lives in `objectiveai-cli`'s
//! `http::laboratories_routes`.

mod wire;
pub use wire::*;
mod listener;
pub use listener::*;
