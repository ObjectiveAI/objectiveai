//! The body: the messages — each the one
//! [`Message`](crate::shared::containers::enqueue::request::Message)
//! an enqueue carries on the provider's wire, named here too.

mod request;

pub use crate::shared::containers::enqueue::request::Message;
pub use request::*;
