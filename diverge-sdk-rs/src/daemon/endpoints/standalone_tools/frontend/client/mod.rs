//! The client side of a frontend request: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is every
//! channel a client may open on it: the body, its end, the cancel.
//! [`execute`] performs the exchange: the answer, and the handle that
//! writes.

pub mod channel_request;
pub mod execute;
pub mod request;
