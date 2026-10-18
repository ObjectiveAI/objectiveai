//! Listing the resources a server offers.
//!
//! A [`request::Request`] opens the channel and a
//! [`response::Frame`] answers it, once. A listing longer than one
//! answer is several exchanges, each carrying the cursor the last one
//! ended with.

pub mod request;
pub mod response;
