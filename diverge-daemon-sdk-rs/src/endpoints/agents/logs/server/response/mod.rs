//! The logs response: the values, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one value, or a
//! failure. [`Item`] is one entry of the log, what the request's
//! program is given and what comes back without one: its id, its
//! time, and what it holds — a [`Kept`] chunk, or an error.

mod frame;
mod item;
mod kept;

pub use frame::*;
pub use item::*;
pub use kept::*;
