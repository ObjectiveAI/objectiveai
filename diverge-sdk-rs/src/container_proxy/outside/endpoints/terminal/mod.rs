//! An interactive shell in the container, on a terminal.
//!
//! The server opens it at a size and gets the terminal's output until
//! the shell ends or the server cancels it; it writes stdin and resizes
//! it on channels, at any time. The shell, its streams and its rules
//! are [`terminal`](crate::shared::containers::terminal)'s.

pub mod client;
pub mod server;
