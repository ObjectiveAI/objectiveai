//! What a server's channel response frame carries on an MCP channel.

/// One MCP answer out of the container: the head once, then as much
/// body as there turns out to be.
///
/// The head is where `Mcp-Session-Id` arrives — which is how a
/// connector LEARNS its session id, since the initialize response
/// mints it — and where `Content-Type` says whether what follows is
/// one JSON document or an event stream held open for the session.
///
/// See [`http::response::Frame`](crate::shared::http::response::Frame).
pub type Frame<'a> = crate::shared::http::response::Frame<'a>;
