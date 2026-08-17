//! The provider half, behind the `server` feature.
//!
//! Everything else in this crate is a message. This is the part that
//! does something with one, and it is opt-in for exactly that reason:
//! a client, and a tool that only inspects the protocol, should not
//! have to compile a provider to read a frame.
//!
//! # Empty
//!
//! Nothing here yet. It lands incrementally, and nothing outside this
//! module changes as it does — a message is the same message whether
//! or not somebody compiled the code that answers it.
