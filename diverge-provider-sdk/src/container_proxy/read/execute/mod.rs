//! Reading a file out, rather than describing the read.
//!
//! [`execute`] opens `/read`, names the file, and hands back an
//! [`ExecuteStream`] of its bytes. Its own files are flattened into
//! it.

mod error;
mod execute;
mod execute_stream;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
