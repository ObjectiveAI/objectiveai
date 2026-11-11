//! Destroying one image in the caller's Store.
//!
//! A caller names an image by its digest and it is gone, with the
//! room it took given back to the Store; or a running container of the caller
//! runs it, and it stays; or the provider could not. Split by who
//! SENDS, as everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
