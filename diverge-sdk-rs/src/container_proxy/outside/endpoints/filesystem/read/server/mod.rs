//! The server side of a read: what the proxy sends.
//!
//! [`response`] is the whole of it — pieces, then the finish, or an
//! error last.
//!
//! There is no `channel_request`. The proxy opens no channels of its
//! own to answer a read; it has everything it needs from the path.
//! And no `channel_response`, because the server opens none for it to
//! answer.

pub mod response;
