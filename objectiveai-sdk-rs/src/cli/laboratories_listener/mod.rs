//! Consumer + wire types for the cli daemon's `/laboratories/{id}`
//! endpoint — one laboratory's full record, live.
//!
//! On connect the daemon sends one
//! [`LaboratoryInstanceEvent::Laboratory`] frame with the current
//! [`LaboratoryRecord`] — the spec (when the lab is connected or in
//! the local scan; zero-filled otherwise), its `source`/`connected`
//! state, and EVERY attachment row targeting it (the AIHs and tags it
//! is attached to) — then re-sends the full record whenever anything
//! about it changes (connect/disconnect, local scan, attachments).
//! Full-value replace, never a patch.
//!
//! The daemon side lives in `objectiveai-cli`'s
//! `http::laboratories_routes`.

mod wire;
pub use wire::*;
mod listener;
pub use listener::*;
