//! The server side of a watch: what a provider sends.
//!
//! [`response`] is the whole of it — a stream, for as long as the
//! scope lives.
//!
//! There is no `channel_request`. A provider opens no channels of its
//! own to answer a watch; it has everything it needs from the name.
//!
//! And no `channel_response`, though a caller does open one — the
//! [`channel_request`](super::client::channel_request) that ends a
//! watch is not answered on its own channel. What answers it is the
//! scope's finish, which is [`response`]'s.

pub mod response;
