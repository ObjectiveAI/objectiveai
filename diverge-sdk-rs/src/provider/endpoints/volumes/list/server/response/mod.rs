//! Volume listing response data.
//!
//! [`Frame`] is what comes back on channel `0` — a volume added,
//! changed or removed, the word that the listing is whole, or a
//! failure to list — and [`Volume`] is what the first three carry.

mod frame;
mod volume;

pub use frame::*;
pub use volume::*;
