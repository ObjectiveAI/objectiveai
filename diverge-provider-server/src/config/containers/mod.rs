//! The `containers` section of `config.yaml`.
//!
//! What the provider gives containers between them: the memory and
//! the disk the running set may reach together, and where their
//! storage is kept. [`Containers`] holds it. More is coming here —
//! this is the section that will grow.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod containers;

pub use containers::*;
