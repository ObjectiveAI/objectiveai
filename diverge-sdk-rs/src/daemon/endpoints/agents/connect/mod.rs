//! MCP as an agent: the scope a client runs an agent's MCP exchanges
//! through, as though the agent's own container ran them.
//!
//! A client names an agent; the daemon judges it and answers exactly
//! one response, that it is connected; then the scope stays open, and
//! the client opens channels on it: one per MCP exchange — the five of
//! [`shared::mcp`](crate::shared::mcp), each answered from the tools
//! served to the agent, the four that answer once with one channel
//! response and the finish, the notifications with one per
//! notification until the scope ends — and the disconnect, which ends
//! the scope. Or the daemon answers that no agent is the one named,
//! forbidden, or that it failed, and finishes.
//!
//! # The exchanges are the agent's
//!
//! The rules:
//!
//! 1. **As the agent.** Every exchange is answered exactly as the same
//!    exchange from the agent's container is: the agent's tools, by
//!    the names the agent sees them by; each request stamped as the
//!    agent's under `_meta`, on the agent's own session with each
//!    tool; each call and read logged in the tool's log as the
//!    agent's. Who opened the scope is not told to any tool.
//! 2. **Deployed as a message would deploy it.** An exchange on an
//!    agent whose container is not up starts it, with every tool it
//!    depends on, and waits for it before it is answered; a container
//!    that will not start is the exchange's error.
//! 3. **Active while in flight.** A list, a call or a read in flight
//!    keeps the agent active, as one from its own container does; the
//!    notifications do not.
//! 4. **The scope holds nothing.** An open scope with nothing in
//!    flight does not hold the agent's container up: its idle clock
//!    runs, and the next exchange starts it again.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the channels are in [`client`] — and the daemon
//! answers, so the answers are in [`server`]. Neither side holds both
//! halves of the exchange.

pub mod client;
pub mod server;
