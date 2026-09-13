//! The answers the proxy sends on the channels the server opened.
//!
//! [`postgres`] is what the container's driver wrote on a database
//! connection. [`register`] is whether the agent was taken;
//! [`run_loop`] the loop, chunk by chunk, [`agent_schema`] what its
//! agent may be, and [`enqueue`] and [`dequeue`] the fates of what
//! the server put in the running loop's queue — the family's own,
//! each but the registration an alias of the shape
//! [`shared::containers`](crate::shared::containers) defines.

pub mod agent_schema;
pub mod dequeue;
pub mod enqueue;
pub mod postgres;
pub mod register;
pub mod run_loop;
