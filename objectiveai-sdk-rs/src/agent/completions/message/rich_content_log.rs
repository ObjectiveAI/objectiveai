//! `RichContentLog` — on-disk shape of message content, where
//! every chunk is a [`LogReference`] to a file on disk.
//!
//! Mirrors [`super::RichContent`]'s untagged Text/Parts split in
//! structure only; the inner types are uniformly references:
//!
//! - `Reference(LogReference)` — the message had a single
//!   [`super::RichContent::Text`] payload; that string was written
//!   to one `.txt` file and we hold the reference.
//! - `Parts(Vec<LogReference>)` — the message had a
//!   [`super::RichContent::Parts`] payload; every part was written
//!   to its own file (text → `.txt`, decodable media → native
//!   binary extension, non-decodable parts → `.json` containing
//!   the serialized [`super::RichContentPart`]) and we hold one
//!   reference per part, in original order.
//!
//! Wire JSON shapes (untagged-distinguishable: one is an object,
//! the other is an array):
//!
//! - `Reference`: `{"type":"reference","path":"…/text/abc-0.txt"}`
//! - `Parts`:     `[{"type":"reference","path":"…"}, …]`
//!
//! The reader contract is informal: open the file at `path`, branch
//! on its extension. `.txt` is raw UTF-8 text → reconstruct as
//! [`super::RichContentPart::Text`]. Binary media files are the
//! decoded blob in their native format. `.json` files deserialize
//! back into [`super::RichContentPart`] verbatim. The grouping
//! subdir (`text|image|audio|video|file`) is a debug hint, not
//! load-bearing for parsing.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::filesystem::logs::LogReference;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.message.RichContentLog")]
pub enum RichContentLog {
    #[schemars(title = "Reference")]
    Reference(LogReference),
    #[schemars(title = "Parts")]
    Parts(Vec<LogReference>),
}
