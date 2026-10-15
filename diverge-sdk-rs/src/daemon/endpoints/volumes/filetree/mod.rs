//! Seeing what a volume holds: one snapshot of its tree.
//!
//! A client names a volume and a subtree of it, empty for the whole;
//! the daemon takes the volume at rest and asks the provider for the
//! tree, as the provider protocol's `volumes::filetree` answers it, and
//! answers with it once, that no volume is the one named or nothing is
//! at the path, that what is there is a file, that the volume is held,
//! forbidden, or that it failed, and the scope finishes. Nothing is
//! watched and no change is sent; a caller that wants to know what
//! changed asks again, and compares. A volume a running container has,
//! or a download, an upload or a transfer is on, is held, and asked
//! about again once it is free.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
