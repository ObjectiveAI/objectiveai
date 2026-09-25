//! What a client sends back on a write, on the one channel the
//! daemon opens: the content. See [`Frame`].

mod frame;

pub use frame::*;
