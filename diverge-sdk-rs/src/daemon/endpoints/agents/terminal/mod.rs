//! An interactive shell in an agent's container, on a terminal.
//!
//! A client names an agent and the terminal's size; the daemon starts a
//! shell on a terminal in the container and sends what it prints, as
//! [`terminal`](crate::shared::containers::terminal) states it, until
//! the shell ends — then how it ended, and the finish; or answers that
//! nothing is the one named, forbidden, or that it failed, and
//! finishes. The terminal is the scope: while it is open the client
//! writes the shell's stdin and sets its size on channels, at any time,
//! and its close — or the client gone — kills the shell. An agent's container that is not running is started for the terminal, and held up while it is open.
//! Split by who SENDS: the request and the channels are in [`client`],
//! the output in [`server`].

pub mod client;
pub mod server;
