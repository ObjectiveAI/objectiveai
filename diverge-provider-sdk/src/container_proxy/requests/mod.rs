//! `/requests`: every ask the container makes.
//!
//! One WebSocket, dialed by the server, carrying
//! [`request::Frame`]s from the container and nothing back: the
//! answer to each comes on its own path, named by the channel the
//! frame carried — see [the module](super).

pub mod request;
