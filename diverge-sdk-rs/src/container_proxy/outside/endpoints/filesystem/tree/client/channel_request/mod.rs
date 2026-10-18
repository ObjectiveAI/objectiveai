//! The channels the server opens on the proxy during a tree.
//!
//! One, and it ends the scope that opened it — see [`Frame`].
//!
//! It is a struct rather than an enum of one, which is the rule
//! everywhere else in this crate: a discriminant with nothing to
//! discriminate is not worth the match.

mod frame;

pub use frame::*;
