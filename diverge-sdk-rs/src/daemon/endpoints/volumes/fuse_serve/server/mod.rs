//! The server side of a volume FUSE serve: what the daemon sends.
//!
//! [`response`] is the one answer on the main stream.
//! [`channel_response`] is what answers each channel a caller opens:
//! the provider's serve's own answers, as the provider gave them. The
//! daemon opens no channel on the scope.

pub mod channel_response;
pub mod response;
