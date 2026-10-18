//! Destroying one image the caller built.
//!
//! A caller names an image and it is gone, with the room it took
//! given back to the store; or a running container of the caller
//! runs it, and it stays; or the provider could not. Split by who
//! SENDS, as everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
