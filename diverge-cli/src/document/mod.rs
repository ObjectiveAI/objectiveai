//! The three documents in the root, as the CLI reads, rewrites and
//! edits them.
//!
//! The root holds one document per program — `daemon.yaml`,
//! `provider.yaml`, `cli.yaml` — and this module handles all three the
//! same way: a document is [`read`] as its program reads it, the
//! built-in defaults where there is no file, and held as JSON on this
//! side; a jq program is [`run`](crate::jq::run) over it, for a `get`
//! to narrow it,
//! for a `set` to rewrite it, and for an `edit` — the [`program`] a jq
//! path expression and a YAML [`value`] make — to set or delete one
//! place in it; what a `set` or an `edit` yields is
//! [`validate`](Which::validate)d by the document's own type — the
//! daemon's `Config`, the provider's, the CLI's — so that a key the
//! program would refuse, or a value of the wrong kind, is refused here
//! and nothing is written; and what passes is [`write()`]ten as the
//! type's own YAML, whole, in place.
//!
//! # What a rewrite costs
//!
//! The file written is the type's serialization of the value: what
//! the document's program reads, exactly, and nothing else. A comment
//! in the file is gone after a `set` or an `edit`, and the keys come
//! out in the type's own order; the CLI's daemons and accounts keep
//! the order the document gave them.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod edit;
mod error;
mod read;
mod which;
mod write;

pub use edit::*;
pub use error::*;
pub use read::*;
pub use which::*;
pub use write::*;
