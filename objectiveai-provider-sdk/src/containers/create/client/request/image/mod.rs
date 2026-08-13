//! Where a container's image comes from.
//!
//! [`Image`] is the choice; [`Client`], [`Server`] and [`Registry`]
//! are what each choice carries.

mod client;
mod image;
mod registry;
mod server;

pub use client::*;
pub use image::*;
pub use registry::*;
pub use server::*;
