//! The logs response: the values, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one value, or a
//! failure. [`ItemWrapper`] is one entry of the log, what the
//! request's program is given and what comes back without one: its
//! index, its time, and the [`Item`] it holds — a chunk, or an
//! [`Error`].

mod error;
mod frame;
mod item;
mod item_wrapper;

pub use error::*;
pub use frame::*;
pub use item::*;
pub use item_wrapper::*;
