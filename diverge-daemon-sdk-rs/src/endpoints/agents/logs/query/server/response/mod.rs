//! The query response: the items, one each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Item`], or a
//! failure to read any. [`Item`] is one entry of the log: its id, its
//! time, the key of the message it belongs to where it has one, and
//! what it holds — a [`Kept`] chunk, or an error.

mod frame;
mod item;
mod kept;

pub use frame::*;
pub use item::*;
pub use kept::*;
