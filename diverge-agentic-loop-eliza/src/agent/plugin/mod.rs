//! Plugin definitions — what the caller adds to the runtime.

mod plugin;
mod rotates;
mod secret;

pub use plugin::*;
pub use rotates::*;
pub use secret::*;
