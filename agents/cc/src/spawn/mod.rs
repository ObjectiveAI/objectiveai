//! The Claude Code subprocess: spawning it, and the enqueue verb
//! against it.
//!
//! One run at a time is one subprocess, behind two locks:
//! [`writer::WRITER`] is stdin — holding it is the sole right to
//! write, and its `None` is the run closed — and [`echoes::ECHOES`]
//! is the batches written and not yet echoed, which the reader takes
//! out by uuid as Claude Code echoes each one, yielding its user
//! parts at the position the message entered the conversation.
//!
//! The rules, in order:
//!
//! 1. An enqueue is answered the moment its write lands: the write
//!    IS the delivery. Nothing waits for Claude Code to take the
//!    message, and nothing withdraws it — Claude Code takes it from
//!    its own queue at a seam of its choosing.
//! 2. An enqueue checks the run open, registers its batch among the
//!    echoes and writes, all under one writer hold; the run's close
//!    clears the writer under the same lock, so nothing is written
//!    into a run that has ended, and an enqueue after the close is
//!    refused as closed.
//! 3. The echoes' lock is held for a push, a removal or a clearing,
//!    never across a write: the reader takes it on every replay echo
//!    and must never wait on an enqueue blocked on a full stdin pipe.
//!
//! Error-typed records — a rejected rate limit, a failed auth
//! status, an error result, a line that failed the parse — always
//! travel as the stream's [`error::Error`], verdict unattached:
//! FATALITY IS FINALITY, the consumer's to decide by what follows.
//! An error before the run's first chunk is the request's own
//! failure (HTTP); one that anything at all follows was survivable
//! news (a non-fatal notification); the one the stream ends ON is
//! the run's death (the fatal final chunk).
//!
//! Deadlock audit: the reader never touches the writer while reading
//! — only at end of stream, and then one lock at a time — and holds
//! the echoes only for a removal, so stdout always drains while an
//! enqueue blocks on a full stdin pipe, and no lock-order cycle
//! exists: the one nested hold is an enqueue's, writer then echoes.

mod content;
mod echoes;
mod enqueue;
mod error;
mod install;
mod session_id;
mod spawn;
mod stdin;
mod writer;

pub use content::blocks;
pub use enqueue::{Closed, enqueue};
pub use install::installed;
pub use session_id::{remember, session_id};
pub use spawn::spawn;
