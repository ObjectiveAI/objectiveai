//! The identities the issuer has issued tokens to.
//!
//! An identity is a subject — the `sub` of every token issued to it —
//! and nothing more: no name and no password. It comes to be when the
//! issuer's operator first approves a device code that asks for it (see
//! [`approve`](crate::store::Devices::approve)), and is listed and
//! removed by the CLI, on the issuer's own host. Removing one removes
//! every code and refresh token issued to it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod identities;
mod identity;
mod list;
mod remove;

pub use identities::*;
pub use identity::*;
