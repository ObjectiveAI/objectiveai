//! The server side of an image build: what the daemon sends.
//!
//! [`response`] is the whole of it. The daemon relays the build's
//! output as it comes and answers how it ended; it opens no channels
//! of its own for this, so there is no `request` here.

pub mod response;
