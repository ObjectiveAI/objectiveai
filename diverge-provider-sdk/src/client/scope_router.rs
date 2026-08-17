//! One scope, and its frames routed by channel.
//!
//! Everything a request opened: the answers on channel `0`, the
//! channels this end opened inside it, and the channels the far end
//! opened back.
//!
//! # Empty
//!
//! Nothing here yet. One thing about its shape is settled.
//!
//! **A scope is not only answers.** The far end opens channels inside
//! it — MCP and Postgres in an
//! [`agentic_loop`](crate::endpoints::agentic_loop), the image and the
//! authorize question in a
//! [`laboratories::run`](crate::endpoints::laboratories::run) — and
//! they have to be answered. So what a scope yields is not a stream of
//! responses; it is a stream of both, in the order they arrived.
//!
//! Which is why they share one stream rather than two. A laboratory's
//! authorize question lands among filetree frames, and a runner
//! answering out of order is answering about the wrong connector.
