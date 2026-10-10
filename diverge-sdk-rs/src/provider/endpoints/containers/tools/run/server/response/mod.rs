//! What a provider sends back on a tool container run: the id, then
//! every connector coming and going, or a failure. See [`Frame`].

mod connector;
mod frame;

pub use connector::*;
pub use frame::*;
