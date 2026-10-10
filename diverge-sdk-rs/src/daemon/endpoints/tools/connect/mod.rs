//! Serving a tool the daemon runs to another daemon: the scope that
//! daemon's agents reach the tool through, over the daemon connection.
//!
//! A client — another daemon, come through a provider as a client of
//! this one — names a tool of the caller's on record; the daemon
//! starts its container if it does not run, holds it while the scope
//! is open, and answers exactly one response, that it is connected;
//! then the scope stays open, and the client opens channels on it:
//! one per MCP exchange — the five of
//! [`shared::mcp`](crate::shared::mcp), each answered by the tool's
//! server inside the container, the four that answer once with one
//! channel response and the finish, the notifications with one per
//! notification until the scope ends — and the disconnect, which ends
//! the scope. The finish follows the disconnect, the tool's run
//! ending, however it ends, or the client's connection ending. Or the
//! daemon answers that no tool of the caller's is the one named,
//! forbidden, or that it failed, and finishes: a dependency tool is
//! its agent's and a connected tool is another daemon's, and naming
//! either is a failure. A tool is connected to by its name or by its
//! template and index, a record tool only.
//!
//! # The scope holds the tool
//!
//! While the scope is open the tool is in use by it, as it is by an
//! active agent it is attached to: it does not stop while the scope is
//! open unless it is deleted or its container ends, and a scope opened
//! on a tool already running is a use of it. What the client sends in
//! a request's `_meta` reaches the tool as sent; what the tool answers
//! is attested by this daemon under `_meta`, as every answer of its
//! own containers is, and the client passes that on.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the channels are in [`client`] — and the daemon
//! answers, so the answers are in [`server`]. Neither side holds both
//! halves of the exchange.

pub mod client;
pub mod server;
