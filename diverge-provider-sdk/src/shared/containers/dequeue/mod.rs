//! Clearing a running loop's queue.
//!
//! The caller opens a channel that carries nothing — there is no
//! request type, the whole queue being the only thing there is to
//! clear — and the provider answers once — with one
//! [`response::Frame`] saying whether the queue held anything — then
//! the finish. Every message withdrawn is also answered on its own
//! [`enqueue`](crate::shared::containers::enqueue) channel, as
//! dequeued; a message the agent already took stays taken, because
//! dequeuing is not un-delivery.

pub mod response;
