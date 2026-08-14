//! The agentic loop — a provider driving an agent through its turns.
//!
//! Split by who SENDS, then by which LEVEL. [`client`] is everything a
//! client puts on the wire; [`server`] is everything a provider does.
//!
//! Within each, a module is named for the frame that carries it, so
//! the modules and [`frame`](crate::frame)'s types say the same words:
//!
//! | module | frame | what it is |
//! |--------|-------|------------|
//! | `request` | `Request` | opens the scope |
//! | `response` | `Response` | the answer, on channel `0` |
//! | `channel_request` | `ChannelRequest` | opens a channel inside it |
//! | `channel_response` | `ChannelResponse` | the answer on one |
//!
//! A side has only the halves it sends. A client opens the scope and
//! answers the channels a server opens; a server answers the scope and
//! opens channels of its own.

pub mod client;
pub mod server;
