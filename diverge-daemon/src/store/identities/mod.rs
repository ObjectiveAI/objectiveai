//! What the daemon keeps of the host's identities: their tags, and
//! nothing else.
//!
//! An identity is a file under the root's `identities/`, signed in on
//! the host and never recorded here; what the daemon adds to one is the
//! tags a client puts on it, kept by its issuer and its subject — the
//! one row there is per tagged identity. [`of_identity`] and [`all`]
//! load; [`set_tags`] writes an identity's tags whole, and an identity
//! whose tags are all taken off has no row. An identity removed from the
//! host keeps its row, and one signed in again under the same issuer and
//! subject finds those tags on it: it is the same identity.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod tags;

pub use tags::*;
