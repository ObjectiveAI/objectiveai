//! The container's asks, carried to the caller and answered back.
//!
//! Every ask arrives on the one `/requests` connection as a channel
//! number and a payload, and is answered on that number's path. Here
//! each becomes a channel this end opens on the scope — the family's
//! frame for it — whose answers become the path's messages. One task
//! per ask, so an answer that takes a while holds up nothing else,
//! and the asks stream ending is the container gone.

mod command;
mod fuse;
mod mcp;
mod one;
mod postgres;
mod relay;
mod vault;

pub(crate) use relay::*;
