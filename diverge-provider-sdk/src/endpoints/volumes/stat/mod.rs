//! Examining one volume.
//!
//! [`list`](super::list) says which volumes exist; this says what is
//! in one. A caller names a volume and gets the listing's three fields
//! back with two more on top — how much of it is used, and the hash
//! of its content. Split by who SENDS, as everywhere else: the ask is
//! in [`client`], the answer in [`server`].
//!
//! # Why these two fields are not in the listing
//!
//! Because each costs a walk of the volume — a size to sum, a manifest
//! to hash — and a listing that paid for every volume's walk would
//! pay for the ones nobody asked about. A caller pays for them here,
//! one volume at a time, and only for the volume it asked about.

pub mod client;
pub mod server;
