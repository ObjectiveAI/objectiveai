//! The answers the proxy sends on the channels the server opened.
//!
//! [`postgres`] is what the container's driver wrote on a database
//! connection, [`schema`] what its arguments may be. [`enqueue`] and
//! [`dequeue`] are the fates of what the server sent the agent — the
//! family's own, each an alias of the shape
//! [`shared::containers`](crate::shared::containers) defines. What
//! the agent says is not here: it rides the begin's own main stream,
//! [`response`](super::response).

pub mod dequeue;
pub mod enqueue;
pub mod postgres;
pub mod schema;
