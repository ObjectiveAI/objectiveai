//! The images a caller builds on this provider: the protocol's
//! `ImageManager`, supplied here as a refusal until the store exists.
//!
//! The protocol is in place — the four store endpoints are read,
//! dispatched and answered — and this provider keeps no such store
//! yet: every listing, stat, build and delete is answered with
//! [`Error::Unsupported`], a failure to serve and never a listing of
//! nothing. The store, its capacity, and the checker's and the
//! deployer's reading of it as the caller's own are the next step.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod image_manager;

pub use error::*;
pub use image_manager::*;
