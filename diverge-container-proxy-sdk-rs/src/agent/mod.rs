//! The agent container's own server: what the proxy dials.
//!
//! An agent container's program is an HTTP server on the container's
//! loopback, at [`port()`], and the proxy beside it is its only
//! caller. This module is that server's contract — the five calls,
//! their bodies, their answers — stated once, for the program that
//! answers them and the proxy that makes them. A tool container has
//! no server here, and nothing dials one: the provider's server knows
//! which kind of container it made.
//!
//! | the proxy calls | with | the program answers |
//! |-----------------|------|---------------------|
//! | `POST /register` | the [`register::request::Request`] JSON | `2xx`, the agent held for the container's life; or a non-`2xx` |
//! | `POST /run` | the [`run::request::Request`] JSON | `2xx` as `text/event-stream`, every `data:` one `AgenticLoopChunk` JSON, the stream's end the loop ended; or a non-`2xx` |
//! | `GET /schema` | nothing | `2xx` with the JSON Schema of the agent value; or a non-`2xx` |
//! | `POST /enqueue` | the [`enqueue::request::Request`] JSON | `2xx` with one [`enqueue::Fate`], held until the fate is known; or a non-`2xx` |
//! | `POST /dequeue` | `{}` | `2xx` with one [`dequeue::Outcome`]; or a non-`2xx` |
//!
//! # Registration comes first, and once
//!
//! The agent is fixed for the container's life. The proxy registers
//! it exactly once, before the first loop; the program refuses a
//! `/run` before that (`{"kind":"unregistered"}`) and refuses any
//! second `/register`, whatever it carries (`{"kind":"registered"}`,
//! `409`). Every loop after runs as that agent, on its own prompt,
//! each resuming the conversation the last one left.
//!
//! # The stream never carries an error
//!
//! `/run` answers one of two things: a non-`2xx`, when there is no
//! loop to report on — the agent value the image will not take, a
//! key it does not have, a history it cannot open, a run already in
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
//! when none does, the waiting messages joined into one prompt. The
//! program's own queue is what a loop holds between seams. A
//! `missed` from the program, or a non-`2xx`, hands the message back
//! to the proxy, which waits for the loop to end and starts the next
//! on it — so no message the provider enqueued is lost to a loop
//! ending.
//!
//! # The port is the container's
//!
//! [`DEFAULT_PORT`], `8080`, unless the environment names another in
//! [`PORT_VARIABLE`] — `PORT`, the name every platform already uses —
//! and names a valid one. The program binds [`port()`]; the proxy
//! dials it; both read the same environment, so the number is never
//! written in two places.

pub mod dequeue;
pub mod enqueue;
pub mod register;
pub mod run;

/// The port the program listens on when the environment does not say
/// otherwise.
pub const DEFAULT_PORT: u16 = 8080;

/// The environment variable that names another port.
pub const PORT_VARIABLE: &str = "PORT";

/// The port the program listens on, and the proxy dials.
///
/// The rules, in order:
///
/// 1. [`PORT_VARIABLE`] set, and its value a decimal number in
///    `1..=65535`: that number. Surrounding whitespace is not part of
///    a number, and is not tolerated.
/// 2. Otherwise — unset, empty, not a number, `0`, or out of range —
///    [`DEFAULT_PORT`].
///
/// Read every time it is called: the environment is the source, and
/// nothing here caches it.
pub fn port() -> u16 {
    std::env::var(PORT_VARIABLE)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_PORT)
}
