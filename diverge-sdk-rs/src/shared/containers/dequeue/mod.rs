//! Withdrawing messages from the agent's queue, by key.
//!
//! The caller opens a channel with [`request::Request`] — a key, as
//! an [`enqueue`](crate::shared::containers::enqueue) gave one — and
//! the provider withdraws every message enqueued under that key and
//! not yet taken, answering once — with one [`response::Frame`]
//! saying whether any was — then the finish. Every message withdrawn
//! is also answered on its own enqueue channel, as dequeued; a
//! message under another key is not touched; a message the agent
//! already took stays taken, because dequeuing is not un-delivery.

pub mod request;
pub mod response;
