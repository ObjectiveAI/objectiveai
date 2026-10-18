//! What the proxy sends back on a begin: that it has begun, with what
//! the program declared, the two words about its activity, or a
//! failure. See [`Frame`], and [`Registered`] for what `Begun`
//! carries.

mod frame;
mod registered;

pub use frame::*;
pub use registered::*;
