//! The server side of a message: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon answers once, when the
//! message's fate is known, and is done; it opens no channels of its
//! own, and it answers nothing on the client's cancel channel, so
//! there is no `request` and no `channel_response` here.

pub mod response;
