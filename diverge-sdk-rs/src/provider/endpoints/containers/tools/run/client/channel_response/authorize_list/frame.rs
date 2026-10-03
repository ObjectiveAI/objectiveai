//! What a client's response frame carries on a listing authorize
//! channel.

/// Yes or no, as for a connector. See
/// [`authorize::response::Frame`](crate::shared::containers::authorize::response::Frame).
///
/// An alias rather than a re-export, for the reason
/// [`authorize_connect`](super::super::authorize_connect) gives: the path says this
/// scope's answer lives here, and it does.
pub type Frame = crate::shared::containers::authorize::response::Frame;
