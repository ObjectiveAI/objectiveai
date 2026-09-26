//! Volume stat response data.
//!
//! [`Frame`] is what comes back on channel `0` — the volume examined,
//! or a failure to examine it — and [`Stat`] is what the first
//! carries.

mod frame;
mod stat;

pub use frame::*;
pub use stat::*;
