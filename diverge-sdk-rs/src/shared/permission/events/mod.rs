//! Over the events: the one graph of its kind.
//!
//! The daemon keeps exactly one graph of who sent what to whom, so a
//! grant over it names nothing and reaches no narrower than the
//! whole: on the wire a bare array of the [`Action`]s held,
//! `{"events":["list"]}`, and holding the action is the whole of it,
//! as it is for a making action. [`Permission`] is that array as a
//! type.

mod action;
mod permission;

pub use action::*;
pub use permission::*;
