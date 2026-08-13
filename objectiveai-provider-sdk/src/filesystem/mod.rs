//! Filesystem — what a provider will let a caller look at, and
//! watching it.
//!
//! [`list`] says which directories exist to be watched; [`watch`]
//! names one and opens a scope that streams its tree. The two are
//! halves of one exchange, which is why a watch names a directory
//! rather than describing one: a caller can only ask for what it was
//! offered.

pub mod list;
pub mod watch;
