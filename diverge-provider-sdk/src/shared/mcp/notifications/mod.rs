//! Hearing what a server says on its own account.
//!
//! A [`request::Request`] opens the channel and carries nothing, and
//! [`response::Frame`]s arrive on it for as long as it lives.

pub mod request;
pub mod response;
