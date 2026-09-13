//! The client side of a write: what the server sends.
//!
//! [`request`] opens it — the destination, once. [`channel_response`]
//! answers the one channel the proxy opens: the content.
//!
//! There is no `channel_request`. The server asks nothing more of a
//! write it started; the content is an answer, not an ask.

pub mod channel_response;
pub mod request;
