//! Pulling an image the caller serves.
//!
//! A [`request::Request`] opens the channel carrying one registry
//! request, and [`response::Frame`]s answer it until the channel
//! finishes.
//!
//! # It is bytes, and this crate reads none of them
//!
//! What rides here is HTTP, and it stays HTTP: a request line, headers,
//! and a response with a status and headers. Nothing between the two
//! ends parses any of it.
//!
//! Which is the opposite of what [`mcp`](super::mcp) does, and the
//! reason is the difference between the two protocols. MCP over
//! Streamable HTTP is JSON-RPC riding HTTP: take the HTTP away and a
//! method name and its params are still there, so five named exchanges
//! lost nothing. The OCI distribution specification has nothing
//! underneath. Its semantics ARE HTTP — `Range` resumes an interrupted
//! layer, `Accept` picks a manifest out of a multi-arch index,
//! `Docker-Content-Digest` confirms what arrived, and a `206` means
//! something a `200` does not.
//!
//! So a typed form would be this crate re-specifying somebody else's
//! specification and then owning it forever. Nineteen endpoints, of
//! which we use two.
//!
//! # What the bytes cost, and what they buy
//!
//! They cost visibility: nothing here can say what is on the wire, the
//! same way nothing can say what is on a
//! [`postgres`](crate::shared::containers::postgres) conduit.
//!
//! They buy correctness that a structured form kept getting wrong. A
//! relay that takes a request apart and builds a new one has to decide
//! what to do about `Content-Length`, `Transfer-Encoding`, `Connection`
//! and `Host` — every one of which describes the message rather than
//! the request, and every one of which is a bug when copied. Bytes are
//! not re-emitted, so there is nothing to get wrong.
//!
//! It also means a caller is free to do what a proxy does: rewrite a
//! `Location` that points at a CDN it can reach and the runtime cannot,
//! answer as an open registry while using its own credentials upstream,
//! follow a redirect and inline the body. None of that is expressible
//! when the shape is fixed here.
//!
//! # One channel is one exchange
//!
//! The request is one frame, whole, and the answer streams back. Which
//! works because a PULL has no request body: `GET` for manifests and
//! blobs, `HEAD` to probe, and `Range` is a header. There is nothing
//! going up but a request line and headers.
//!
//! A push would break that — a blob is arbitrarily large and would not
//! fit a frame. Nothing here pushes, and whoever changes that is
//! changing this shape rather than adding a case to it.

pub mod request;
pub mod response;
