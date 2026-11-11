//! The `messages` section of `provider.yaml`.
//!
//! One thing: the [`Messages`] rate, the fixed window at which the
//! provider relays direct messages between the identities connected to
//! it. The section absent, the provider relays none.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod messages;

pub use messages::*;
