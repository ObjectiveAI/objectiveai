//! An interactive shell in a dependency tool's container, on a terminal.
//!
//! A client names a dependency tool and the terminal's size; the daemon starts a
//! shell on a terminal in the container and sends what it prints, as
//! [`terminal`](crate::shared::containers::terminal) states it, until
//! the shell ends — then how it ended, and the finish; or answers that
//! nothing is the one named, forbidden, or that it failed, and
//! finishes. The terminal is the scope: while it is open the client
//! writes the shell's stdin and sets its size on channels, at any time,
//! and its close — or the client gone — kills the shell. A dependency tool that is not running is not found: nothing is started for it. One that runs is held up — with the agent it was deployed for — while the terminal is open.
//! Split by who SENDS: the request and the channels are in [`client`],
//! the output in [`server`].

pub mod client;
pub mod server;
