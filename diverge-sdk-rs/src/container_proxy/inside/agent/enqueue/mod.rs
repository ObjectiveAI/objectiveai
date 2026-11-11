//! `POST /enqueue`: messages for the running loop, accepted.
//!
//! The proxy sends the [`request::Request`] JSON — every message it
//! holds, in order, each under its key — while a loop runs,
//! one at a time, and the program answers at once: `204`, the
//! messages on its queue, taken at its next seam as ONE user turn —
//! with whatever else waits there — their parts each under their own
//! key. The `204` is the messages' delivery: the proxy answers every
//! one of them `delivered` on it, and nothing waits for the turn.
//!
//! `409` `{"kind":"closed"}` is the run closed under the batch — the
//! loop's last look found the queue empty and ended it — and the one
//! `409` the program answers here: the proxy puts the batch back at
//! the front of its queue and starts the next loop on it. Any other
//! `4xx` is the batch refused — content the program will not take,
//! checked before it accepts anything — and every message in it is
//! answered with the error. A `5xx`, or no answer, is the batch the
//! proxy's again, like a `409`.

pub mod request;
