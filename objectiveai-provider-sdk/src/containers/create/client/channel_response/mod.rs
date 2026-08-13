//! What a caller sends back during a creation.
//!
//! [`http::response`](crate::http::response) under a shorter name, not
//! a copy of it. A provider's registry endpoint relays what a caller's
//! own registry answers, so the answer is an HTTP response and there
//! is nothing here that is not one.
//!
//! Which is what makes `404` mean something. A blob a caller cannot
//! produce is a blob that is not there, said the way HTTP already says
//! it — no separate signal, and no ambiguity with the empty layer,
//! whose digest is real and whose body is legitimately zero bytes.

pub use crate::http::response as response;
