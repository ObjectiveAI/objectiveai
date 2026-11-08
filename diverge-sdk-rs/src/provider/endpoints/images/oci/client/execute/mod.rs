//! Performing an image pull, rather than describing it.
//!
//! [`execute`] opens the scope of one image and hands back an
//! [`ExecuteHandle`]: one method per ask —
//! [`manifest`](ExecuteHandle::manifest), [`blob`](ExecuteHandle::blob)
//! — each its own channel, and [`cancel`](ExecuteHandle::cancel) to
//! end the scope. The readers are the run's: the same shared frames
//! answer the same two asks there, and [`answered`] reads them once
//! for both.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

pub mod answered;

mod error;
mod execute;
mod execute_handle;

pub use error::*;
pub use execute::*;
pub use execute_handle::*;
