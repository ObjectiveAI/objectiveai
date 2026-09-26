//! The agent container's own server: what the proxy dials.
//!
//! An agent container's program is an HTTP server on the container's
//! loopback, at [`port()`](crate::container_proxy::inside::port), and the proxy beside it is
//! its only caller. This module is that server's contract — the
//! loop's three calls, their bodies, their answers — stated once,
//! for the program that answers them and the proxy that makes them,
//! beside the two calls every container answers:
//! [`register`](crate::container_proxy::inside::register) and `GET /schema`.
//!
//! | the proxy calls | with | the program answers |
//! |-----------------|------|---------------------|
//! | `POST /register` | the [`register::request::Request`](crate::container_proxy::inside::register::request::Request) JSON | `2xx` with the [`register::response::Response`](crate::container_proxy::inside::register::response::Response) JSON, the tools the program depends on, the arguments held for the container's life; or a non-`2xx` |
//! | `POST /run` | the [`run::request::Request`] JSON | `2xx` as `text/event-stream`, every `data:` one `AgenticLoopChunk` JSON — the messages' user parts first — the stream's end the loop ended; or a non-`2xx` |
//! | `GET /schema` | nothing | `2xx` with the JSON Schema of the arguments; or a non-`2xx` |
//! | `POST /enqueue` | the [`enqueue::request::Request`] JSON | `2xx` with one [`enqueue::Fate`], held until the fate is known; or a non-`2xx` |
//! | `POST /dequeue` | the [`dequeue::request::Request`] JSON | `2xx` with one [`dequeue::Outcome`]; or a non-`2xx` |
//!
//! # Registration comes first, and once
//!
//! The arguments are fixed for the container's life, and so are the
//! tools the program answers with. The proxy registers them exactly
//! once, before the first loop; the program refuses a `/run` before
//! that (`{"kind":"unregistered"}`) and
//! refuses any second `/register`, whatever it carries
//! (`{"kind":"registered"}`, `409`). Every loop after runs as the
//! agent the program made of them, on its own prompt, each resuming
//! the conversation the last one left.
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
//! # The queue is the proxy's, and the program's
//!
//! The proxy holds every message the provider enqueues and offers
//! them to the program one at a time: `POST /enqueue` while a loop
//! runs, held until the program says what became of it; `POST /run`
//! when none does, with the waiting messages, each under its key, in
//! enqueue order. The program's own queue is what a loop holds between seams,
//! and a `POST /dequeue` names a key: every message under it and not
//! yet taken is withdrawn, on the proxy's side and the program's, and
//! every other left waiting. A `missed` from the program, a `5xx`, or no answer hands the
//! message back to the proxy, which waits for the loop to end and
//! starts the next on it — so no message the provider enqueued is
//! lost to a loop ending. A `4xx` is the message refused: content
//! the program will not take, and its fate is the error, in the
//! program's words.

pub mod dequeue;
pub mod enqueue;
pub mod run;
