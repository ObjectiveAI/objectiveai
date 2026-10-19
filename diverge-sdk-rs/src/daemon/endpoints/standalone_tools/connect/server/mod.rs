//! The server side of a connect: what the daemon sends.
//!
//! [`response`] is the one answer on the scope's main stream;
//! [`channel_response`] is what answers each channel the client opens.
//! The daemon opens no channel of its own on a connect, so there is no
//! `channel_request` here.

pub mod channel_response;
pub mod response;
