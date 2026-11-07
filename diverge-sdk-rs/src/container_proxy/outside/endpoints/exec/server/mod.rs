//! The server side of an exec: what the proxy sends.
//!
//! [`response`] is the whole of it — the command's stream. The cancel is
//! not answered on its own channel: the scope's finish answers it.

pub mod response;
