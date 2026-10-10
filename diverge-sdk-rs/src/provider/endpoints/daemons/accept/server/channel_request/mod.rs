//! The one channel the provider opens on an accept scope: its half of
//! a connection, carrying who the connector is.

mod frame;

pub use frame::*;
