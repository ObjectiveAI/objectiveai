//! One thing a client sent that starts something.

use bytes::Bytes;

use super::scope_handle::ScopeHandle;

/// What a [`Session`](super::session::Session) yields: a token, or a
/// request beside the scope that answers it.
///
/// Two variants because a client initiates exactly two things. An
/// [`Auth`](Self::Auth) establishes the connection and a
/// [`Request`](Self::Request) opens a scope on it — everything else a
/// client sends lives inside a scope that already exists, and the
/// session routes it there without yielding anything.
///
/// # A token here is a violation
///
/// The session is built once the
/// [`handshake`](crate::wire::handshake) is done, so a token that
/// arrives after it breaks the rule that each end sends exactly one.
/// The session reports it, in the order it came; the holder ends the
/// connection.
#[derive(Debug)]
pub enum Received {
    /// A token: the payload of an
    /// [`Auth`](crate::wire::frame::client::ClientFrame::Auth) frame, the
    /// header already gone — after the handshake, a violation.
    Auth(Bytes),
    /// A request, beside the scope that answers it.
    ///
    /// The payload is the tag byte and the request's own bytes —
    /// exactly what [`ClientRequest`](crate::provider::endpoints::ClientRequest)
    /// decodes. The request rides beside the handle rather than inside
    /// it because it is read exactly once, by whatever dispatches on
    /// it, and a handle that carried the bytes too would be a copy
    /// nobody reads.
    Request(Bytes, ScopeHandle),
}
