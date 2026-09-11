//! The channels a caller opens on a provider during a watch.
//!
//! One, and it ends the scope that opened it — see [`Frame`].
//!
//! It is a struct rather than an enum of one, which is the rule
//! everywhere else in this crate: a discriminant with nothing to
//! discriminate is not worth the match. The tag byte stays, so a second
//! thing to ask a running watch would be additive rather than a wire
//! break.

mod frame;

pub use frame::*;
