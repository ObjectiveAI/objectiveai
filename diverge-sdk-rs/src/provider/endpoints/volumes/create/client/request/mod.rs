//! Volume creation request data.
//!
//! [`Frame`] is what a caller hands a provider to get a volume: a
//! name, a size, and whether it keeps what is written into it.

mod frame;

pub use frame::*;
