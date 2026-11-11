//! An interactive shell in a container, on a terminal.
//!
//! One shell on a pseudo-terminal: `/bin/bash` where the image has it,
//! else `/bin/sh`, started in `/` with `TERM=xterm-256color` and the
//! terminal as its controlling terminal — so `cd`, variables, jobs and
//! full-screen programs all behave as on any terminal. What it is asked
//! to start at is its [`open::Open`] size; what comes back is its
//! [`output`] stream; what goes in is its [`input`].
//!
//! The rules:
//!
//! 1. The output is one stream, stdout and stderr together, as a
//!    terminal has them: `Stdout` per read of the terminal, then
//!    exactly one `Exited` or `Signaled`; or a lone `Error` when no
//!    shell could start.
//! 2. Input goes in at any time, the output streaming or not: `Stdin`
//!    bytes written to the terminal as they come — a keystroke, a
//!    paste, a control byte like Ctrl-C's `0x03` — in their order, and
//!    `Resize` setting its size.
//! 3. Both are paced by their readers (see
//!    [`credit`](crate::wire::credit)), each on its own: output pouring
//!    out never holds input back, nor input output.

pub mod input;
pub mod open;
pub mod output;
pub mod session;
