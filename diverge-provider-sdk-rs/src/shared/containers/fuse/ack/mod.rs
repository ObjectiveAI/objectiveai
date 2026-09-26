//! The answer to an ask that either happened or did not: ok, the
//! refusal a volume that keeps nothing gives every change, or the
//! error. What [`write`](super::write), [`truncate`](super::truncate),
//! [`setattr`](super::setattr), [`remove`](super::remove),
//! [`rename`](super::rename) and [`mkdir`](super::mkdir) are answered
//! with; [`Refused`] is the owned form a caller answers a mutation
//! with, before it is a frame.

mod frame;
mod refused;

pub use frame::*;
pub use refused::*;
