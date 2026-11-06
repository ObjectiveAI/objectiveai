//! Ending one caller's MCP session.
//!
//! A [`request::Request`] opens the channel and carries nothing but
//! the session it names, and a [`response::Frame`] answers it, once.
//! The rules:
//!
//! 1. The session closed is the one the request's caller names, by its
//!    [`key`](super::session::key); the caller is the whole of the
//!    request, as [`Sessioned`](super::session::Sessioned) carries it.
//! 2. Closing a session ends it: its client to the server cancelled,
//!    and nothing dialled on it again. The caller's next exchange, if
//!    one comes, makes a new session, as a first exchange does.
//! 3. Closing a session that is not open is closing it: the answer is
//!    [`Closed`](response::Frame::Closed) all the same, since the
//!    caller has asked for a state that already holds.
//! 4. The caller's side asks it when the session is over on its side —
//!    the one connect scope a client's session was minted for has
//!    ended, or the agent a session belongs to is deleted — so a tool
//!    container's proxy holds no session for a caller that will never
//!    use it again.
//!
//! # Why it is an exchange of its own
//!
//! Because nothing else says it. A session is opened by its first
//! exchange, and every exchange after it is one more use; the end of
//! the caller is not an exchange with the server at all, and the
//! proxy, which never sees a connect scope, cannot tell a caller that
//! is gone from one that is quiet.

pub mod request;
pub mod response;
