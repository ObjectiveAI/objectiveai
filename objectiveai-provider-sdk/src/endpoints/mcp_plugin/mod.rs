//! MCP plugins — a container that serves tools.
//!
//! The third kind of container, beside an
//! [`agentic_loop`](super::agentic_loop)'s and a
//! [`laboratory`](super::laboratories). An agent runs in the first, an
//! agent works inside the second, and an agent CALLS the third.
//!
//! [`run`] is the one scope, and everything is inside it.
//!
//! # What makes it the odd one out
//!
//! Nothing is injected into it. A laboratory gets an MCP server put
//! inside it by the provider, which is why the provider decides that
//! server's port and hands it a working directory. A plugin image
//! ALREADY serves MCP — its own server is the entrypoint and stays
//! PID 1 — so the provider adds nothing and only has to learn where to
//! dial.
//!
//! Which turns three of a laboratory's fields into questions nobody
//! can answer here. There is no port to assign, because the image's
//! author chose one. There is no working directory to set, because the
//! image's own `WORKDIR` governs the entrypoint and there is no second
//! process to place elsewhere. And there are no mounts, because a
//! plugin serves tools rather than works on a filesystem.
//!
//! It gains two things instead: [`arguments`], which configure the
//! plugin, and an [`identity`], which tells it who is calling.
//!
//! [`arguments`]: run::client::request::Frame::arguments
//! [`identity`]: run::client::request::Frame::identity

pub mod run;
