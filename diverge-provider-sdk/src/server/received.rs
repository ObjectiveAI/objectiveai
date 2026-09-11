//! One thing a client sent that starts something.

use bytes::Bytes;

use super::scope_handle::ScopeHandle;

/// What a [`Session`](super::session::Session) yields: a credential, or
/// a request beside the scope that answers it.
///
/// Two variants because a client initiates exactly two things. An
/// [`Auth`](Self::Auth) establishes the connection and a
/// [`Request`](Self::Request) opens a scope on it — everything else a
/// client sends lives inside a scope that already exists, and the
/// session routes it there without yielding anything.
///
/// # WHEN a credential may arrive is not this type's business
///
/// The session reports what came, in the order it came. That the first
/// frame is the only place a credential belongs — and that a connection
/// the provider dialled should never produce one at all — is the
/// handshake's rule, enforced where the handshake is:
/// [`handle`](super::handle::handle). A session that policed it would
/// be a second copy of the rule, one connection layer below the thing
/// that acts on it.
#[derive(Debug)]
pub enum Received {
    /// A credential: the payload of an
    /// [`Auth`](crate::frame::client::ClientFrame::Auth) frame, the
    /// header already gone.
    ///
    /// The mode byte and the credential's own bytes — exactly what
    /// [`frame::auth::Auth`](crate::frame::auth::Auth) decodes.
    /// Refcounted out of the frame it arrived in, never a copy.
    Auth(Bytes),
    /// A request, beside the scope that answers it.
    ///
    /// The payload is the tag byte and the request's own bytes —
    /// exactly what [`ClientRequest`](crate::endpoints::ClientRequest)
    /// decodes. The request rides beside the handle rather than inside
    /// it because it is read exactly once, by whatever dispatches on
    /// it, and a handle that carried the bytes too would be a copy
    /// nobody reads.
    Request(Bytes, ScopeHandle),
}
