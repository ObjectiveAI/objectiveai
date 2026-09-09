//! The agent container's own server: what the proxy forwards to.
//!
//! An agent container's entrypoint is an HTTP server on the
//! container's loopback, at [`port()`], and the four `/agent/*` paths
//! the provider's server opens on the proxy are each one call to it,
//! forwarded. The proxy holds nothing of the loop's — no schema, no
//! queue, no attachment — and dials this server only when the
//! provider's server has opened a path that needs it. That is what
//! lets one proxy serve both kinds of container: a tool container
//! has no server here, and nothing asks for one, because the
//! provider's server knows which kind it made and only opens the
//! `/agent/*` paths on an agent container.
//!
//! | the proxy's path | it calls | the agent's server answers |
//! |------------------|----------|----------------------------|
//! | `/agent/register` | `POST /register`, body the [`register::request::Request`](super::register::request::Request) JSON | `2xx`, the agent held for the container's life; or a non-`2xx` |
//! | `/agent/run` | `POST /run`, body the [`run_loop::request::Request`](super::run_loop::request::Request) JSON | `2xx` as `text/event-stream`, every `data:` one [`AgenticLoopChunk`](crate::shared::containers::run_loop::response::AgenticLoopChunk) JSON, the stream's end the loop ended; or a non-`2xx` |
//! | `/agent/schema` | `GET /schema` | `2xx` with the JSON Schema of the agent value; or a non-`2xx` |
//! | `/agent/enqueue` | `POST /enqueue`, body the [`enqueue::request::Request`](super::enqueue::request::Request) JSON | `2xx` with one [`enqueue::Fate`], held until the fate is known; or a non-`2xx` |
//! | `/agent/dequeue` | `POST /dequeue`, body `{}` | `2xx` with one [`dequeue::Outcome`]; or a non-`2xx` |
//!
//! # Registration comes first, and once
//!
//! The agent is fixed for the container's life. The server registers
//! it exactly once, before the first loop; the agent's server refuses
//! a `/run` before that (`{"kind":"unregistered"}`) and refuses any
//! second `/register`, whatever it carries (`{"kind":"registered"}`,
//! `409`) — both non-`2xx`, forwarded as the path's `Error`. Every
//! loop after runs as that agent, on its own prompt.
//!
//! # The stream never carries an error
//!
//! `/run` answers one of two things: a non-`2xx`, when there is no
//! loop to report on — the agent value the image will not take, a
//! key it does not have, a history it cannot open, the first fetch,
//! a run already in progress — or a `2xx` stream of chunks. The
//! stream is chunks and only chunks; a loop that fails after it has
//! said something says so as a fatal notification chunk, part of the
//! output, and then ends. So the first item decides: anything that
//! fails before it is the status, anything after it is a chunk.
//!
//! # What a non-2xx becomes
//!
//! Every non-`2xx` is forwarded as the path's `Error` frame. Its body
//! is the error's value when it is JSON — the agent's server speaking
//! in the protocol's one error shape, verbatim — and otherwise the
//! proxy wraps it: `{"kind":"agent","status":<status>,"error":<body
//! as text>}`. A server that cannot be dialed at all — refused,
//! because nothing listens — is `{"kind":"agent","error":<reason>}`,
//! at once: the proxy does not wait for one to appear, since the
//! provider's server only asks an agent container, whose server is
//! its entrypoint.
//!
//! # The port is the container's
//!
//! [`DEFAULT_PORT`], `8080`, unless the environment names another in
//! [`PORT_VARIABLE`] — `PORT`, the name every platform already uses —
//! and names a valid one. The agent's server binds [`port()`]; the
//! proxy dials it; both read the same environment, so the number is
//! never written in two places.

pub mod dequeue;
pub mod enqueue;

/// The port the agent's server listens on when the environment does
/// not say otherwise.
pub const DEFAULT_PORT: u16 = 8080;

/// The environment variable that names another port.
pub const PORT_VARIABLE: &str = "PORT";

/// The port the agent's server listens on, and the proxy dials.
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
