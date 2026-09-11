//! Destroying a volume.
//!
//! A caller names one and it stops existing. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].
//!
//! The counterpart to [`create`](super::create), and the only thing
//! that ends a volume — nothing else in this specification does, and
//! no scope closing or connection dropping ever will.

pub mod client;
pub mod server;
