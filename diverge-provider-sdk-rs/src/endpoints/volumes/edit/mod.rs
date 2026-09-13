//! Changing a volume's reserved size.
//!
//! A caller names one and states a new size. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in [`server`].
//!
//! The size is the only thing about a volume that can be changed. Its
//! name is the handle and everything else is a consequence — what a
//! [`list`](super::list) reports beside it either follows from the
//! contents or was fixed when the volume came into being.

pub mod client;
pub mod server;
