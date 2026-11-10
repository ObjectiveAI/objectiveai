//! The agent container's own server: what the proxy dials.
//!
//! An agent container's program is an HTTP server on the container's
//! loopback, at [`port()`](crate::container_proxy::inside::port), and the proxy beside it is
//! its only caller. This module is that server's contract — the
//! loop's three calls, their bodies, their answers — stated once,
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
//! | `POST /enqueue` | the [`enqueue::request::Request`] JSON, the proxy's queue joined | `2xx` with one [`enqueue::Fate`], the slot's, held until the fate is known; `409` `{"kind":"occupied"}` while the slot is full; or another non-`2xx` |
//! | `POST /dequeue` | the [`dequeue::request::Request`] JSON, `{}` | `2xx` with one [`dequeue::Outcome`], the slot emptied; or a non-`2xx` |
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
//! # The queue is the proxy's; the program holds one slot
//!
//! The proxy holds every message the provider enqueues, and the
//! program holds at most ONE: the rules, in order.
//!
//! 1. The proxy offers its whole queue at once, joined in exact
//!    enqueue order: `POST /run` when no loop runs, `POST /enqueue`
//!    while one does. Either way the messages are one user turn, each
//!    message's user parts under its own key, so the log credits every
//!    sender.
//! 2. `POST /enqueue` fills the slot and is held until the slot's
//!    fate is known. A second while it is full is `409`
//!    `{"kind":"occupied"}`; the proxy never sends one.
//! 3. `POST /dequeue` names no key and empties the slot — its
//!    `/enqueue` answers `dequeued` — or finds it empty. A slot the
//!    loop already took stays taken.
//! 4. A new message for a busy loop, or the provider withdrawing a key
//!    the slot holds, makes the proxy withdraw the slot, then offer
//!    its queue again, joined: what came back first, in its order,
//!    then what is newer — without the withdrawn key's messages.
//! 5. A `missed` from the program, a `5xx`, or no answer hands the
//!    batch back to the proxy, which waits for the loop to end and
//!    starts the next on it — so no message the provider enqueued is
//!    lost to a loop ending. A `4xx` is the batch refused: content the
//!    program will not take, and every message's fate is the error, in
//!    the program's words.

pub mod dequeue;
pub mod enqueue;
pub mod run;
