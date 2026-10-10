//! What a provider sends back on a connect: the connection open, the
//! acceptor's frames, or a failure. See [`Frame`].

mod frame;

pub use frame::*;
