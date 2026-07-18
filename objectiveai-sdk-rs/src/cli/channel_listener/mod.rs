//! The daemon's `/channels` endpoint — duplex channels.

mod wire;
pub use wire::*;
mod listener;
pub use listener::*;
