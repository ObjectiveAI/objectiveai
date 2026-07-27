//! What a plugin gets to decide about serving.
//!
//! Deliberately short. Most of the wire shape is fixed by the
//! laboratory host — see [`crate::serve`] — and a knob for something
//! that can only hold one value is just a way to get it wrong.

use std::time::Duration;

/// How to serve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The port the plugin's manifest declares under `mcp.port`.
    ///
    /// The host publishes exactly that port, so this and the manifest
    /// are two halves that have to agree: a mismatch means the host
    /// publishes a port nothing is listening on, and the plugin looks
    /// dead rather than misconfigured.
    pub port: u16,
    /// How often to ping an idle SSE stream, or `None` to never.
    ///
    /// Worth having because the connection crosses podman's port
    /// publish and whatever sits between: an idle stream can be
    /// reaped by something in the middle that has no idea the plugin
    /// is simply thinking.
    pub sse_keep_alive: Option<Duration>,
}

impl Config {
    /// The config for a plugin whose manifest declares
    /// `mcp.port = port`.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            sse_keep_alive: Some(Duration::from_secs(15)),
        }
    }
}
