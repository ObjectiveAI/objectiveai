//! One HTTP request to the program's frontend, beneath `/frontend`.
//!
//! The server opens it with the request's head and gets the program's
//! response — its head, then its body — until it ends or the server
//! stops it; it sends the request's body on channels. The request, its
//! answer and its rules are [`frontend`](crate::shared::containers::frontend)'s.

pub mod client;
pub mod server;
