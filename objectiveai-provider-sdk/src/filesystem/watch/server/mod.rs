//! The server side of a watch: what a provider sends.
//!
//! [`response`] is the whole of it — a stream, for as long as the
//! scope lives.
//!
//! There is no `channel_request`. A provider opens no channels of its
//! own to answer a watch; it has everything it needs from the name.

pub mod response;
