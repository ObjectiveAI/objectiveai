//! The client side of a connection: what a connector sends.
//!
//! [`request`] opens the scope and asks to be let in.
//! [`channel_request`] reaches into the container once it is.
//!
//! There is no `channel_response`. A provider opens no channels on a
//! connector — a connector supplies no image and authorizes nobody,
//! so there is nothing to ask it for.

pub mod channel_request;
pub mod request;
