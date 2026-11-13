//! Which kinds of content, beside text, an agent takes.

use rmcp::model::{ContentBlock, ResourceContents};
use serde::{Deserialize, Serialize};

use super::Modality;

/// The kinds of content an agent's program takes beside text, each
/// said outright: three members, all required, and no other.
///
/// Text is not among them: every agent takes text, and there is
/// nothing to say. A resource link is not among them either: it is a
/// name and an address, which an agent is handed as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Modalities {
    /// Whether it takes an image: an MCP image block.
    pub image: bool,
    /// Whether it takes audio: an MCP audio block.
    pub audio: bool,
    /// Whether it takes a file: an MCP embedded resource whose
    /// contents are bytes and are neither an image nor audio.
    pub file: bool,
}

impl Modalities {
    /// Whether this takes `modality`.
    pub fn takes(&self, modality: Modality) -> bool {
        match modality {
            Modality::Image => self.image,
            Modality::Audio => self.audio,
            Modality::File => self.file,
        }
    }

    /// The modalities `content` carries that this does not take: empty
    /// when it takes all of it. The rules:
    ///
    /// 1. Text and a resource link carry none: every agent takes
    ///    text, and a link is a name and an address.
    /// 2. An image block carries [`Modality::Image`]; an audio block
    ///    carries [`Modality::Audio`].
    /// 3. An embedded resource whose contents are text carries none.
    ///    One whose contents are bytes carries [`Modality::Image`]
    ///    when its media type begins `image/`, [`Modality::Audio`]
    ///    when it begins `audio/`, and [`Modality::File`] otherwise, a
    ///    resource that states no media type included.
    /// 4. A block, or a resource's contents, of a kind this crate does
    ///    not know carries none: what becomes of it is the agent's
    ///    program's to say.
    /// 5. Each modality not taken is listed once, in the order image,
    ///    audio, file, however many blocks carry it.
    pub fn lacking(&self, content: &[ContentBlock]) -> Vec<Modality> {
        let mut lacking: Vec<Modality> = content.iter().filter_map(carried).filter(|modality| !self.takes(*modality)).collect();
        // Rule 5.
        lacking.sort();
        lacking.dedup();
        lacking
    }
}

/// Rules 1 to 4: the modality one block carries, if any.
fn carried(block: &ContentBlock) -> Option<Modality> {
    match block {
        ContentBlock::Text(_) | ContentBlock::ResourceLink(_) => None,
        ContentBlock::Image(_) => Some(Modality::Image),
        ContentBlock::Audio(_) => Some(Modality::Audio),
        ContentBlock::Resource(embedded) => match &embedded.resource {
            ResourceContents::TextResourceContents { .. } => None,
            ResourceContents::BlobResourceContents { mime_type, .. } => {
                let media = mime_type.as_deref().unwrap_or("");
                if media.starts_with("image/") {
                    Some(Modality::Image)
                } else if media.starts_with("audio/") {
                    Some(Modality::Audio)
                } else {
                    Some(Modality::File)
                }
            }
            _ => None,
        },
        _ => None,
    }
}
