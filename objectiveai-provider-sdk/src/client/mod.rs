//! What a client asks a provider for.
//!
//! Every module here is one thing a client can request and the shape
//! of what comes back. They are siblings rather than layers: running a
//! loop, watching a filesystem and checking an image are independent
//! capabilities, and a provider may serve any subset of them.

pub mod agentic_loop;
pub mod filetree;
pub mod images;
