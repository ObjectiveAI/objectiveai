//! The answer to an ask that either happened or did not: ok, or the
//! error. What [`write`](super::write), [`remove`](super::remove),
//! [`rename`](super::rename) and [`mkdir`](super::mkdir) are answered
//! with.

mod frame;

pub use frame::*;
