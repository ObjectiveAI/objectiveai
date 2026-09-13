//! `POST /run`: one loop, on one prompt.
//!
//! The proxy sends the [`request::Request`] JSON once the agent is
//! registered and no loop runs. A `2xx` is the loop as it happens:
//! `text/event-stream`, every `data:` one `AgenticLoopChunk` JSON,
//! which the proxy relays to the provider verbatim, and the stream's
//! end is the loop ended. A non-`2xx` is a loop that never ran — the
//! agent value the image will not take, a key it does not have, a
//! history it cannot open, a run already in progress — and the proxy
//! answers every message the loop was to take with its words.
//!
//! One loop at a time is the program's rule, stated as its own
//! non-`2xx`; the proxy never starts a second while one runs. A
//! failure after output is a fatal notification chunk, part of the
//! output, then the end: the stream never carries an error event.

pub mod request;
