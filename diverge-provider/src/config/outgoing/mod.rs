//! The `outgoing` section of `provider.yaml`: the peers the provider
//! dials.
//!
//! Each [`Outgoing`] is an address and the identity that must answer
//! there. The provider presents its one `identity` to it. A peer that
//! dials the provider is judged by the document's `incoming` instead:
//! the two lists are apart.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod outgoing;

pub use outgoing::*;
