//! Who is asking, and what their grants reach.
//!
//! Two judgments, and nothing else decides anything. The first is at
//! the handshake: the [`DaemonAuthenticator`] verifies the token the
//! other end presents and [`admit`](DaemonAuthenticator::admit)s the
//! `(issuer, subject)` it establishes as [`Admitted`] — a provider the
//! configuration accepts, an administrator its `admins` names, or the
//! account that identity is, a [`Who`] — or as nothing, which is the
//! socket closed. With no account, no administrator and no accepted
//! provider, nothing is admitted. A tool's `/daemon` connection is
//! admitted by no token: its [`Who`] is what its container
//! [`Declared`] at its registration — the grants, and which tool's
//! attached agents its `attached` reaches. The
//! second is per request: [`Standing`] is who is asking as of THIS
//! request — an account's identity and the union of the grants of
//! the roles it holds, read fresh inside the request's own
//! transaction, or a tool's grants as declared with the agents its
//! `attached` reaches read fresh — and
//! [`accounts`], [`roles`], [`providers`],
//! [`daemons`], [`agents_templates`],
//! [`standalone_tools_templates`], [`dependency_tools`],
//! [`agents`], [`standalone_tools`], [`postgres`], [`vault`], [`events`] and [`volumes`] are the pure functions that say whether a
//! standing holds a making action, holds an action over a record, or
//! holds a tagging action over a record and its tags, by the rule the
//! wire states: any one grant allowing is the whole of it, nothing
//! denies, and a grant's `within` is the kind's own list filter read
//! as a TEST, which [`filter`] is. An administrator's standing holds no
//! grant and needs none: every one of those functions allows it
//! everything, first and without reading a grant — see
//! [`Standing::admin`].
//!
//! # What is not here
//!
//! Nothing loads a record, and nothing writes one. A handler loads
//! what the request names, asks the judge, and acts; the judge sees
//! records as values and answers booleans. That is what keeps the
//! rules in one place and the handlers free of them.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod admit;
mod declared;
mod standing;
mod who;

pub use admit::*;
pub use declared::*;
pub use standing::*;
pub use who::*;

pub mod accounts;
pub mod agents;
pub mod agents_templates;
pub mod dependency_tools;
pub mod events;
pub mod images;
pub mod filter;
pub mod postgres;
pub mod daemons;
pub mod providers;
pub mod roles;
pub mod standalone_tools;
pub mod standalone_tools_templates;
pub mod vault;
pub mod volumes;
