//! The one message on the path: ok, or error — the
//! [`ack`](super::super::ack).

/// Ok, or why not.
pub type Frame<'a> = super::super::ack::Frame<'a>;
