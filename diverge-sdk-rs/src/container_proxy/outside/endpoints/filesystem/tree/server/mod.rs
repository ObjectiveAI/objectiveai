//! The server side of a tree: what the proxy sends.
//!
//! [`response`] is the whole of it — a stream, for as long as the
//! scope lives.
//!
//! There is no `channel_request`. The proxy opens no channels of its
//! own to answer a tree; it has everything it needs from the request.
//!
//! And no `channel_response`, though the server does open one — the
//! [`channel_request`](super::client::channel_request) that ends a
//! tree is not answered on its own channel. What answers it is the
//! scope's finish, which is [`response`]'s.

pub mod response;
