//! What the daemon keeps of a volume: its tags, and nothing else.
//!
//! A volume is its provider's, listed by the provider and never
//! recorded here; what the daemon adds to one is the tags a client
//! puts on it, kept by the volume's provider and name — the one row
//! there is per tagged volume. [`of_volume`] and [`all`] load;
//! [`set_tags`] writes a volume's tags whole, and a volume whose tags
//! are all taken off has no row; [`forget`] drops the row when the
//! daemon deletes the volume. A volume deleted on its provider by
//! somebody else keeps its row, and a volume the provider later lists
//! under the same name finds those tags on it: the daemon cannot tell
//! the two apart, and says so here.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod tags;

pub use tags::*;
