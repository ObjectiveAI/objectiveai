//! Performing a serve, rather than describing it.
//!
//! [`execute`] opens the scope naming the volume, waits for the
//! provider to say it is serving, and hands back an
//! [`ExecuteHandle`]: one [`ask`](ExecuteHandle::ask) at a time, each
//! its own channel and its own one answer, and [`stop`](ExecuteHandle::stop)
//! to end the scope.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_handle;

pub use error::*;
pub use execute::*;
pub use execute_handle::*;
