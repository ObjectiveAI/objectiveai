//! `POST /run`: one loop, on one or more messages, as one user turn.
//!
//! The proxy sends the [`request::Request`] JSON once the arguments
//! are registered and no loop runs: every message it holds, in
//! index order — joined by the program into the loop's first user
//! turn, exactly as an [`enqueue`](super::enqueue) is. A `2xx` is the loop as it happens:
//! `text/event-stream`, every `data:` one `AgenticLoopChunk` JSON,
//! which the proxy relays to the provider verbatim, and the stream's
//! end is the loop ended. A non-`2xx` is a loop that never ran —
//! arguments the image will not take, a key it does not have, a
//! history it cannot open, a run already in progress — and the proxy
//! answers every message the loop was to take with its words.
//!
//! The `2xx` is also the messages' delivery: the proxy answers every
//! one of them `delivered` on it, so the program answers once it has
//! taken them, before the turn's work.
//!
//! One loop at a time is the program's rule, stated as its own
//! non-`2xx`; the proxy never starts a second while one runs. A
//! failure after output is a fatal notification chunk, part of the
//! output, then the end: the stream never carries an error event.

pub mod request;
