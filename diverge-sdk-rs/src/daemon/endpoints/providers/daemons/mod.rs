//! Daemons: other daemons the caller holds accounts on, reached
//! through a provider both are connected to.
//!
//! A daemon record is a name of the caller's choosing and its
//! [`Link`]s: one per provider of the caller's the remote daemon is
//! reachable through, naming the provider by the identity this daemon
//! knows it by and the remote daemon by the identity that provider
//! knows IT by. On it are its [`accounts`]: zero or more, each a name
//! of the caller's choosing and a [`Mode`](super::outgoing::Mode) —
//! how this daemon authenticates to the remote in as that account,
//! the same modes as an outgoing provider's, unbrokered with a
//! credential the remote daemon was told to expect as an incoming
//! credential of its own. Every connection to another daemon goes
//! through a provider — the provider protocol's `daemons::connect`,
//! on a provider a link names, naming the remote by the link's
//! identity and presenting an account's mode — and the remote judges
//! the credential as it judges any client's. This daemon holds one
//! connection per account at a time, reused while it lasts, and opens
//! one, when it holds none, through the links whose providers it is
//! connected to, in random order, the first to answer winning. What a
//! daemon record is FOR is acting as this daemon on that one, always as
//! one of its accounts, which the caller must hold a grant to act as:
//! a connected tool —
//! [`tools::register`](crate::daemon::endpoints::standalone_tools::register)
//! names the record, the account and a tool as that daemon names it,
//! which this daemon reaches through that daemon's
//! [`tools::connect`](crate::daemon::endpoints::standalone_tools::connect)
//! over the connection — and a relayed request, any request of this
//! wire carrying a `relay` naming the record and the account, which
//! this daemon puts to that one as that account and carries the
//! answers of. [`add`] names one; [`get`] answers one as a list would;
//! [`list`] lists them, narrowed, with their accounts by name and
//! their tags; [`delete`] forgets one no connected tool names;
//! [`edit`] replaces its links; [`tag`] and [`untag`] change its tags,
//! which are the caller's; [`accounts`] adds, rotates and forgets its
//! accounts.

mod account;
mod link;

pub use account::*;
pub use link::*;

pub mod accounts;
pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
