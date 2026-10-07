//! Who is asking, and what their grants reach.
//!
//! Two judgments, and nothing else decides anything. The first is at
//! the handshake: [`admit_peer`] turns a credential and the address
//! it came from into a [`Peer`] — a [`Who`], the account the
//! connection is served for, or the identity of a provider that
//! dialled in — or into nothing, which is the socket closed; see
//! [`key`] for how a key is minted and how it is kept as a hash. The
//! second is per request: [`Standing`] is the account as of THIS
//! request — its identity and the union of the grants of the roles it
//! holds, read fresh inside the request's own transaction — and
//! [`accounts`], [`roles`], [`providers_outgoing`],
//! [`providers_incoming`], [`agents_templates`], [`tools_templates`]
//! and [`resources`] are the pure functions that say whether a
//! standing holds a making action, holds an action over a record, or
//! holds a tagging action over a record and its tags, by the rule the
//! wire states: any one grant allowing is the whole of it, nothing
//! denies, and a grant's `within` is the kind's own list filter read
//! as a TEST, which [`filter`] is.
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
mod standing;
mod who;

pub use admit::*;
pub use standing::*;
pub use who::*;

pub mod accounts;
pub mod agents_templates;
pub mod filter;
pub mod key;
pub mod providers_incoming;
pub mod providers_outgoing;
pub mod resources;
pub mod roles;
pub mod tools_templates;
