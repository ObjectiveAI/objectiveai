//! The body: nothing.
//!
//! Not the provider protocol's dequeue — that names a key, and stays
//! the provider wire's. The program holds one message, so there is
//! nothing to name: `{}` empties the slot, whatever it holds.

mod request;

pub use request::*;
