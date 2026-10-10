//! Exposing a tool the daemon runs to another daemon: what a
//! connecting daemon's [`connect`](super::connect) opens here, over its
//! daemon connection, to join the tool.
//!
//! A client names a tool of the caller's on record; the daemon starts
//! its container if it does not run — one already running is touched,
//! its idle clock reset — and answers exactly one response: the
//! provider the container runs on, as this daemon names it, the
//! identity this daemon is known by there when it accepts daemon
//! connections through it, the container's id, and an authorization
//! minted for this exposure alone, which the provider protocol's
//! `containers::tools::connect` presents and which admits exactly one
//! connection; then nothing more, and the scope stays open for as long
//! as the tool's run lasts: the finish is the run ending, however it
//! ends. The client's cancel, the one channel it opens, lets the
//! exposure go, and a tool held by nothing else then stops. Or the
//! daemon answers that no tool of the caller's is the one named,
//! forbidden, or that it failed, and finishes: a dependency tool is
//! read-only, and a connected tool is another daemon's, exposed there,
//! and either is a failure.
//!
//! # The exposure holds the tool
//!
//! While the scope is open the tool is in use by it, as it is by an
//! active agent it is attached to: it does not go idle for want of
//! agents, and it does not stop while the scope is open unless it is
//! deleted or its container ends. The authorization is answered here
//! and never again, held in memory and on no record, and is spent by
//! the one connection that presents it; the scope ending, however it
//! ends, is its end too.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the cancel are in [`client`] — and the daemon answers,
//! so the answer is in [`server`]. Neither side holds both halves of
//! the exchange.

pub mod client;
pub mod server;
