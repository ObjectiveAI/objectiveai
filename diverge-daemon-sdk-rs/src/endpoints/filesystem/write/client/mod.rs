//! The client side of a filesystem write: what a client sends.
//!
//! [`request`] opens it — the destination, once. [`channel_response`]
//! answers the one channel the daemon opens: the content.
//!
//! There is no `channel_request`. A client asks nothing more of a
//! write it started; the content is an answer, not an ask.

pub mod channel_response;
pub mod request;
