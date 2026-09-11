//! How far a volume may grow.
//!
//! A caller names a volume and gets one number: how many bytes, as of
//! now, its size could be increased by. Split by who SENDS, as
//! everywhere else: the question is in [`client`], the answer in
//! [`server`].
//!
//! # Headroom, not a size
//!
//! The number is what can be ADDED. An [`edit`](super::edit) states an
//! absolute size, so a caller that wants the largest size it could ask
//! for adds this to the volume's
//! [`bytes`](super::list::server::response::Volume::bytes).
//!
//! # It reserves nothing
//!
//! The number is a fact about the instant it was answered. An
//! [`edit`](super::edit) within it may still be answered insufficient
//! capacity if the room went elsewhere in between, and that answer is
//! the one that counts.

pub mod client;
pub mod server;
