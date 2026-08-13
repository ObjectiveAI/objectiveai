//! The client side of the agentic loop: what a client sends.
//!
//! [`request`] opens the scope. [`channel_response`] is what it sends
//! back on the channels the server opens inside that scope.
//!
//! There is no `response` here and no `channel_request`. A client does
//! not answer its own request, and it opens no channels of its own in
//! a loop — the chunks come back from the server, and the tunnels are
//! the server's to ask for.

pub mod channel_response;
pub mod request;
