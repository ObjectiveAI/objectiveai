//! What a provider sends back on a listing: a container added or
//! removed, one at a time, the word that the listing is whole, or a
//! failure. See [`Frame`].

mod container;
mod frame;

pub use container::*;
pub use frame::*;
