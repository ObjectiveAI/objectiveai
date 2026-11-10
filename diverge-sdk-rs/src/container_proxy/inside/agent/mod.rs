//! The agent container's own server: what the proxy dials.
//!
//! An agent container's program is an HTTP server on the container's
//! loopback, at [`port()`](crate::container_proxy::inside::port), and the proxy beside it is
//! its only caller. This module is that server's contract — the
//! loop's two calls, their bodies, their answers — stated once,
//! for the program that answers them and the proxy that makes them,
//! beside the calls every container answers:
//! [`register`](crate::container_proxy::inside::register) and the
//! two over its [`secrets`](crate::container_proxy::inside::secrets).
//! What the arguments may be is not among them: that is the image's
//! `/DIVERGE.md`, which the proxy reads from the root and the program
//! is never asked for.
//!
//! | the proxy calls | with | the program answers |
//! |-----------------|------|---------------------|
//! | `POST /register` | the [`register::request::Request`](crate::container_proxy::inside::register::request::Request) JSON | `2xx` with the [`register::response::Response`](crate::container_proxy::inside::register::response::Response) JSON, the dependencies the program declares and the secrets it takes, the arguments and the template id held for the container's life; or a non-`2xx` |
//! | `POST /secrets` | the [`secrets::feed::request::Request`](crate::container_proxy::inside::secrets::feed::request::Request) JSON | `2xx` once the values are held, its body unread; or a non-`2xx` |
//! | `POST /secrets/collect` | the [`secrets::collect::request::Request`](crate::container_proxy::inside::secrets::collect::request::Request) JSON | `2xx` with the [`secrets::collect::response::Response`](crate::container_proxy::inside::secrets::collect::response::Response) JSON, every name asked for with its value; or a non-`2xx` |
//! | `POST /run` | the [`run::request::Request`] JSON | `2xx` as `text/event-stream`, every `data:` one `AgenticLoopChunk` JSON — the messages' user parts first — the stream's end the loop ended; or a non-`2xx` |
//! | `POST /enqueue` | the [`enqueue::request::Request`] JSON, the proxy's queue joined | `204` at once, the messages accepted; `409` `{"kind":"closed"}` once the loop's last look has closed the run; or another `4xx`, content the program will not take, checked before accepting |
//!
//! # Registration comes first, and once
//!
//! The arguments are fixed for the container's life, and so are the
//! dependencies the program answers with and the modes it declares
//! its secrets in. The proxy registers them exactly once, before the
//! first loop; the program refuses a `/run` before that
//! (`{"kind":"unregistered"}`) and refuses any second `/register`,
//! whatever it carries (`{"kind":"registered"}`, `409`). The secrets'
//! values are not fixed: they are fed on `POST /secrets` once the
//! registration is answered and again at every activation — a loop
//! about to start — and the `rotating` ones are collected on
//! `POST /secrets/collect` at every inactive — a loop ended — so a
//! value the program changed in a loop is the value the next loop is
//! fed. Every loop runs as the agent the program made of the
//! arguments, on its own prompt, each resuming the conversation the
//! last one left.
//!
//! # The stream never carries an error
//!
//! `/run` answers one of two things: a non-`2xx`, when there is no
//! loop to report on — arguments the image will not take, a key it
//! does not have, a history it cannot open, a run already in
//! progress — or a `2xx` stream of chunks. The stream is chunks and
//! only chunks; a loop that fails after it has said something says
//! so as a fatal notification chunk, part of the output, and then
//! ends. So the first item decides: anything that fails before it is
//! the status, anything after it is a chunk.
//!
//! And the first item is the messages' user parts, said the moment
//! the program has taken them — handed to the harness, the turn on
//! its way — and never after the turn's work: `/run` answers once the
//! messages are taken, because its `2xx` is what the proxy reports as
//! their `delivered`. A turn one that fails after it — a model that
//! will not answer, a script that raises — is a failure after the
//! stream started, a fatal notification chunk like any later turn's.
//!
//! # Delivered means handed to the program
//!
//! The proxy holds every message the provider enqueues until the
//! program has it, and the program keeps a plain queue of what it
//! accepted, which nothing withdraws: the rules, in order.
//!
//! 1. The proxy sends its whole queue at once, joined in order —
//!    the batches as they reached it, each batch's messages as the
//!    batch carried them: `POST /run` when no loop runs, `POST
//!    /enqueue` while one does — one `/enqueue` at a time, so the
//!    order holds.
//!    Either way the messages are one user turn, each message's user
//!    parts under its own key, so the log credits every sender.
//! 2. `POST /enqueue` answers at once. `204` is the messages
//!    accepted: on the program's queue, taken at its next seam, and
//!    every one of them `delivered` on the provider's wire — as is a
//!    `2xx` from the `/run` that carried them. Nothing waits for a
//!    turn.
//! 3. The program takes everything queued at each seam as ONE user
//!    turn — however many `/enqueue`s brought it — each message's
//!    user parts under its own key.
//! 4. The loop's last look takes what is queued or, finding nothing,
//!    closes the run in the same breath; an `/enqueue` after it is
//!    `409` `{"kind":"closed"}`, the one `409` it answers, and the
//!    batch goes back to the front of the proxy's queue to start the
//!    next loop — so no message the provider enqueued is lost to a
//!    loop ending. A `5xx` or no answer does the same.
//! 5. Another `4xx` is the batch refused: content the program will
//!    not take, checked before it accepts anything, and every
//!    message's fate is the error, in the program's words.

pub mod enqueue;
pub mod run;
