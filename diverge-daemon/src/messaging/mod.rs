//! Messaging: the daemon's own sends of direct messages, through its
//! providers.
//!
//! A provider holds one send per connection and counts every sending
//! identity's sends against a fixed window; the daemon is one identity
//! at each of its providers, so it keeps, per provider, the one lane its
//! sends take turns on and the window they are counted in:
//! [`Messaging`], used by
//! [`providers::daemons::message`](crate::serve::providers::daemons::message).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod messaging;

pub use messaging::*;
