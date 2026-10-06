//! Deleting a volume. One request, one answer. A client names a volume;
//! the daemon answers that the volume is gone, that no volume is the
//! one named, that it is in use and was left as it is, forbidden, or
//! that it failed, and the scope finishes. In use is any agent or tool
//! of the daemon's naming it in its mounts, running or not, or a
//! download, an upload or a transfer on it now — see
//! [`volumes`](super). Gone is gone: the volume no longer exists, and a
//! caller that wanted the space back with the name kept deletes and
//! creates.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
