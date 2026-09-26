//! The server side of an image check: what a provider sends.
//!
//! [`response`] is the whole of it. A provider answers and is done; it
//! opens no channels of its own for a question this small, so there is
//! no `request` here the way there is on the agentic loop's server
//! side.
//!
//! # And a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose caller
//! it is, and an
//! [`ImageChecker`](crate::provider::server::image_checker::ImageChecker), and it
//! answers.

pub mod response;

pub mod handle;
