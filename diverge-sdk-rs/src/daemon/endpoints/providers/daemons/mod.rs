//! Daemons: other daemons the caller holds an account on, reached
//! through a provider both are connected to.
//!
//! A daemon record is a name of the caller's choosing, a
//! [`Mode`](super::outgoing::Mode) — how this daemon authenticates to
//! that one, the same modes as an outgoing provider's, unbrokered with
//! a credential the remote daemon was told to expect as an incoming
//! credential of its own — and its [`Link`]s: one per provider of the
//! caller's the remote daemon is reachable through, naming the provider
//! by the identity this daemon knows it by and the remote daemon by the
//! identity that provider knows IT by. Every connection to another
//! daemon goes through a provider — the provider protocol's
//! `daemons::connect`, on a provider a link names, naming the remote by
//! the link's identity and presenting the mode — and the remote judges
//! the credential as it judges any client's. This daemon holds one
//! connection to a daemon at a time, reused while it lasts, and opens
//! one, when it holds none, through the links whose providers it is
//! connected to, in random order, the first to answer winning. What a
//! daemon record is FOR is
//! [`tools::connect`](crate::daemon::endpoints::tools::connect): a tool
//! of another daemon's, named by the record and as that daemon names
//! it, which this daemon joins through that daemon's
//! [`tools::expose`](crate::daemon::endpoints::tools::expose). [`add`]
//! names one; [`get`] answers one as a list would; [`list`] lists them,
//! narrowed, with their tags; [`delete`] forgets one no connected tool
//! names; [`edit`] replaces its mode, which is how a credential
//! rotates, or its links; [`tag`] and [`untag`] change its tags, which
//! are the caller's.

mod link;

pub use link::*;

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
