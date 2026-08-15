//! Volumes — what a provider will let a caller look at, and watching
//! it.
//!
//! [`list`] says which volumes exist; [`watch`] names one and opens a
//! scope that streams its tree. The two are halves of one exchange,
//! which is why a watch names a volume rather than describing one: a
//! caller can only ask for what it was offered.
//!
//! # Volumes rather than paths
//!
//! A volume is a directory a provider has DECIDED to offer, under a
//! [`name`](list::server::response::Volume::name) it chose. That is
//! the whole access model: a caller never states a host path, because
//! there is no path it could state that a provider would resolve. It
//! names something it was given and descends from there, and `..` is
//! just a name in a component list rather than an instruction.
//!
//! Which is also what they are FOR. A
//! [`Mount`](crate::endpoints::laboratories::create::client::request::Mount)
//! names one of these and makes it visible inside a laboratory, so
//! what a caller can watch and what it can mount are one list rather
//! than two that could disagree.

pub mod list;
pub mod watch;
