//! Performing a tools connect: the exchange, rather than the
//! description of it.
//!
//! [`execute`] opens the scope and reads the one answer; what it hands
//! back is an [`ExecuteHandle`], the scope held, with every channel a
//! client may open on it as a method.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod execute;
mod execute_handle;

pub use execute::*;
pub use execute_handle::*;
