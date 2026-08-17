# diverge-provider-sdk

Wire types for the Diverge provider API.

This crate is the **normative artifact** of the provider
specification. The Rust types defined here *are* the protocol. Prose
documents the requirements a provider must satisfy; it does not define
the messages.

## Scope

Types only, by default. No transport, no client, no server. That is
deliberate: both sides of the protocol depend on this crate, as do
tools that only inspect it, so the default build carries no runtime
concerns.

## Features

| feature | what it adds |
|---------|--------------|
| `server` | the provider half |
| `client` | the caller half |

Both are off unless asked for, and both bring `connection`, which
carries either kind of socket: one this process accepted, or one it
dialled. Which end dialled is a fact about TCP rather than about the
protocol — a provider usually waits to be connected to and sometimes
connects out, and a caller can be connected to just as well.

The dependencies are cut to naming those two socket types and nothing
more: `axum` without `http1` or `http2`, and `tokio-tungstenite` with
`stream` alone. Nothing here serves HTTP or dials. A provider stands up
its own server, or connects with its own client, and hands over what
comes out.

Rules about *sequences* of messages — ordering, cardinality, what may
appear first or last — are properties of a stream rather than of any
one message, so no type declaration can carry them. They live in the
prose specification instead, enforced by construction in the provider
frameworks.

## Status

Under construction. The types land as the provider API is defined.

## License

MIT
