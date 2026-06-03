//! `SimpleContentLog` — on-disk shape of [`super::SimpleContent`].
//!
//! Parallels [`super::RichContentLog`] (post-rewrite): every chunk is
//! a [`LogReference`] to its own `.txt` file. No inline content.
//!
//! Two variants, matching `SimpleContent`'s untagged Text/Parts split:
//!
//! - `Reference` — the message had a single
//!   [`super::SimpleContent::Text`] payload; that string was written
//!   to one `.txt` file and we hold the reference.
//! - `Parts` — the message had a [`super::SimpleContent::Parts`]
//!   payload; every part (always
//!   [`super::SimpleContentPart::Text`]) was written to its own
//!   `.txt` file and we hold one reference per part, in original
//!   order.
//!
//! On the wire (untagged-distinguishable: object vs array):
//!
//! - `Reference`: `{"type":"reference","path":"…/text/abc-0.txt"}`
//! - `Parts`:     `[{"type":"reference","path":"…"}, …]`

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::LogReference;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.message.SimpleContentLog")]
pub enum SimpleContentLog {
    #[schemars(title = "Reference")]
    Reference(LogReference),
    #[schemars(title = "Parts")]
    Parts(Vec<LogReference>),
}
