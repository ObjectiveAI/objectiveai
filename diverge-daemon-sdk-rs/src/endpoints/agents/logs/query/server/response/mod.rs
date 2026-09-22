//! The query response: the values the program yielded, one each, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one value the jq program
//! yielded, or a failure. [`Item`] is what the program is given, one
//! per entry of the log, and what a program of `.` yields: its id, its
//! time, and what it holds — a [`Kept`] chunk, or an error.

mod frame;
mod item;
mod kept;

pub use frame::*;
pub use item::*;
pub use kept::*;
