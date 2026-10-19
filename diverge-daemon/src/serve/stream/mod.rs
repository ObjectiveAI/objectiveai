//! A list kept open: what every record list shares once it is a
//! stream.
//!
//! A [`Source`] is the list as the handler would send it now — judged,
//! filtered, in the store's order; [`listing`] subscribes to the word
//! that the kind changed, reads the source, sends every item as added
//! and the word that the listing is whole, and from then on reads the
//! source again at every word and sends the [`diff`] — each
//! [`Change`](diverge_sdk::shared::listing::Change), the SDK's, as
//! the endpoint's own frame, which every list frame is `From` —
//! until the client cancels or its connection ends. The handler
//! finishes the scope after.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod diff;
mod listing;
mod source;

pub use diff::*;
pub use listing::*;
pub use source::*;
