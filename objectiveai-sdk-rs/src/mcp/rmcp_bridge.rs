//! Conversions between our own MCP types and the external [`rmcp`]
//! crate's model types (rmcp 1.7).
//!
//! HARD RULE: our own mcp type is always the middle-man. There is no
//! direct objectiveai <-> rmcp conversion; every path is
//! `objectiveai -> mcp -> rmcp` or `rmcp -> mcp -> objectiveai`.
//!
//! - **Layer 1** (`From`, both ways): each own mcp type that has a base
//!   conversion gets a foundation `From` to/from its rmcp counterpart —
//!   `ContentBlock <-> rmcp Content`, `ImageContent <-> rmcp ImageContent`,
//!   `AudioContent <-> rmcp AudioContent`, `ResourceContentsUnion <-> rmcp
//!   ResourceContents`. The forward direction mirrors the (now-superseded)
//!   `objectiveai-mcp` bridge field-for-field.
//! - **Layer 2** (delegating): for every existing base<->mcp conversion,
//!   a base<->rmcp conversion that composes Layer 1 with the base<->mcp
//!   impl. Each is a one-liner.
//!
//! Round-trip losses (inherent to rmcp's model, pre-existing in the old
//! bridge): rmcp's `RawAudioContent` has no `_meta`, and resource-link
//! `icons` are dropped (rmcp's `Icon` differs from ours and is outside
//! the base<->mcp conversion set).

use indexmap::IndexMap;
use serde_json::Value;

use rmcp::model::{
    Annotated, AudioContent as RmcpAudioContent, Content as RmcpContent,
    ImageContent as RmcpImageContent, Meta, RawAudioContent, RawContent, RawEmbeddedResource,
    RawImageContent, RawResource, RawTextContent, ResourceContents as RmcpResourceContents,
};

use crate::agent::completions::message::{
    File, ImageUrl, InputAudio, RichContent, RichContentPart, VideoUrl,
};
use crate::mcp::shared::{
    BlobResourceContents, ResourceContents, ResourceContentsUnion, TextResourceContents,
};
use crate::mcp::tool::{
    AudioContent, ContentBlock, EmbeddedResource, ImageContent, ImageUrlNotDataUrl, ResourceLink,
    TextContent,
};

// ── _meta helpers (ordered both ways) ────────────────────────────────────────

/// Our `Option<IndexMap>` → rmcp `Option<Meta>` (a `serde_json::Map`),
/// preserving insertion order.
fn sdk_meta_to_rmcp(meta: Option<IndexMap<String, Value>>) -> Option<Meta> {
    meta.map(|m| {
        let mut map = serde_json::Map::with_capacity(m.len());
        for (k, v) in m {
            map.insert(k, v);
        }
        Meta(map)
    })
}

/// rmcp `Option<Meta>` → our `Option<IndexMap>` (inverse of
/// [`sdk_meta_to_rmcp`]).
fn rmcp_meta_to_sdk(meta: Option<Meta>) -> Option<IndexMap<String, Value>> {
    meta.map(|Meta(map)| map.into_iter().collect::<IndexMap<String, Value>>())
}

// ── Layer 1: own mcp <-> rmcp ─────────────────────────────────────────────────

impl From<ContentBlock> for RmcpContent {
    fn from(block: ContentBlock) -> Self {
        let raw = match block {
            ContentBlock::Text(t) => RawContent::Text(RawTextContent {
                text: t.text,
                meta: sdk_meta_to_rmcp(t._meta),
            }),
            ContentBlock::Image(i) => RawContent::Image(RawImageContent {
                data: i.data,
                mime_type: i.mime_type,
                meta: sdk_meta_to_rmcp(i._meta),
            }),
            // rmcp `RawAudioContent` has no `meta`, so `a._meta` is dropped.
            ContentBlock::Audio(a) => RawContent::Audio(RawAudioContent {
                data: a.data,
                mime_type: a.mime_type,
            }),
            ContentBlock::EmbeddedResource(er) => RawContent::Resource(RawEmbeddedResource {
                resource: er.resource.into(),
                meta: sdk_meta_to_rmcp(er._meta),
            }),
            // our `icons` are dropped (rmcp `Icon` differs; out of scope).
            ContentBlock::ResourceLink(rl) => RawContent::ResourceLink(RawResource {
                uri: rl.uri,
                name: rl.name,
                title: rl.title,
                description: rl.description,
                mime_type: rl.mime_type,
                size: None,
                icons: None,
                meta: sdk_meta_to_rmcp(rl._meta),
            }),
        };
        Annotated {
            raw,
            annotations: None,
        }
    }
}

impl From<RmcpContent> for ContentBlock {
    fn from(content: RmcpContent) -> Self {
        match content.raw {
            RawContent::Text(t) => ContentBlock::Text(TextContent {
                text: t.text,
                annotations: None,
                _meta: rmcp_meta_to_sdk(t.meta),
            }),
            RawContent::Image(i) => ContentBlock::Image(ImageContent {
                data: i.data,
                mime_type: i.mime_type,
                annotations: None,
                _meta: rmcp_meta_to_sdk(i.meta),
            }),
            RawContent::Audio(a) => ContentBlock::Audio(AudioContent {
                data: a.data,
                mime_type: a.mime_type,
                annotations: None,
                _meta: None,
            }),
            RawContent::Resource(er) => ContentBlock::EmbeddedResource(EmbeddedResource {
                resource: er.resource.into(),
                annotations: None,
                _meta: rmcp_meta_to_sdk(er.meta),
            }),
            RawContent::ResourceLink(rl) => ContentBlock::ResourceLink(ResourceLink {
                name: rl.name,
                uri: rl.uri,
                title: rl.title,
                description: rl.description,
                mime_type: rl.mime_type,
                icons: None,
                annotations: None,
                _meta: rmcp_meta_to_sdk(rl.meta),
            }),
        }
    }
}

impl From<ImageContent> for RmcpImageContent {
    fn from(i: ImageContent) -> Self {
        Annotated {
            raw: RawImageContent {
                data: i.data,
                mime_type: i.mime_type,
                meta: sdk_meta_to_rmcp(i._meta),
            },
            annotations: None,
        }
    }
}

impl From<RmcpImageContent> for ImageContent {
    fn from(i: RmcpImageContent) -> Self {
        ImageContent {
            data: i.raw.data,
            mime_type: i.raw.mime_type,
            annotations: None,
            _meta: rmcp_meta_to_sdk(i.raw.meta),
        }
    }
}

impl From<AudioContent> for RmcpAudioContent {
    fn from(a: AudioContent) -> Self {
        // rmcp `RawAudioContent` has no `meta`; `a._meta` is dropped.
        Annotated {
            raw: RawAudioContent {
                data: a.data,
                mime_type: a.mime_type,
            },
            annotations: None,
        }
    }
}

impl From<RmcpAudioContent> for AudioContent {
    fn from(a: RmcpAudioContent) -> Self {
        AudioContent {
            data: a.raw.data,
            mime_type: a.raw.mime_type,
            annotations: None,
            _meta: None,
        }
    }
}

impl From<ResourceContentsUnion> for RmcpResourceContents {
    fn from(rcu: ResourceContentsUnion) -> Self {
        match rcu {
            ResourceContentsUnion::Text(t) => RmcpResourceContents::TextResourceContents {
                uri: t.base.uri,
                mime_type: t.base.mime_type,
                text: t.text,
                meta: sdk_meta_to_rmcp(t.base._meta),
            },
            ResourceContentsUnion::Blob(b) => RmcpResourceContents::BlobResourceContents {
                uri: b.base.uri,
                mime_type: b.base.mime_type,
                blob: b.blob,
                meta: sdk_meta_to_rmcp(b.base._meta),
            },
        }
    }
}

impl From<RmcpResourceContents> for ResourceContentsUnion {
    fn from(rc: RmcpResourceContents) -> Self {
        match rc {
            RmcpResourceContents::TextResourceContents {
                uri,
                mime_type,
                text,
                meta,
            } => ResourceContentsUnion::Text(TextResourceContents {
                base: ResourceContents {
                    uri,
                    mime_type,
                    _meta: rmcp_meta_to_sdk(meta),
                },
                text,
            }),
            RmcpResourceContents::BlobResourceContents {
                uri,
                mime_type,
                blob,
                meta,
            } => ResourceContentsUnion::Blob(BlobResourceContents {
                base: ResourceContents {
                    uri,
                    mime_type,
                    _meta: rmcp_meta_to_sdk(meta),
                },
                blob,
            }),
        }
    }
}

// ── Layer 2: base <-> rmcp (delegating; one per master-list row) ──────────────

// A1: ImageUrl -> ImageContent (fallible) -> rmcp ImageContent
impl TryFrom<ImageUrl> for RmcpImageContent {
    type Error = ImageUrlNotDataUrl;
    fn try_from(v: ImageUrl) -> Result<Self, Self::Error> {
        Ok(ImageContent::try_from(v)?.into())
    }
}

// A2: InputAudio -> AudioContent -> rmcp AudioContent
impl From<InputAudio> for RmcpAudioContent {
    fn from(v: InputAudio) -> Self {
        AudioContent::from(v).into()
    }
}

// A3: RichContentPart -> ContentBlock -> rmcp Content
impl From<RichContentPart> for RmcpContent {
    fn from(v: RichContentPart) -> Self {
        ContentBlock::from(v).into()
    }
}

// A4: ImageUrl -> ContentBlock -> rmcp Content
impl From<ImageUrl> for RmcpContent {
    fn from(v: ImageUrl) -> Self {
        ContentBlock::from(v).into()
    }
}

// A5: InputAudio -> ContentBlock -> rmcp Content
impl From<InputAudio> for RmcpContent {
    fn from(v: InputAudio) -> Self {
        ContentBlock::from(v).into()
    }
}

// A6: VideoUrl -> ContentBlock -> rmcp Content
impl From<VideoUrl> for RmcpContent {
    fn from(v: VideoUrl) -> Self {
        ContentBlock::from(v).into()
    }
}

// A7: File -> ContentBlock -> rmcp Content
impl From<File> for RmcpContent {
    fn from(v: File) -> Self {
        ContentBlock::from(v).into()
    }
}

// A8: RichContent -> Vec<ContentBlock> -> Vec<rmcp Content>
impl From<RichContent> for Vec<RmcpContent> {
    fn from(v: RichContent) -> Self {
        Vec::<ContentBlock>::from(v)
            .into_iter()
            .map(RmcpContent::from)
            .collect()
    }
}

// B1: rmcp ResourceContents -> ResourceContentsUnion -> RichContentPart
impl From<RmcpResourceContents> for RichContentPart {
    fn from(v: RmcpResourceContents) -> Self {
        ResourceContentsUnion::from(v).into()
    }
}

// B2: rmcp Content -> ContentBlock -> RichContentPart
impl From<RmcpContent> for RichContentPart {
    fn from(v: RmcpContent) -> Self {
        ContentBlock::from(v).into()
    }
}

// B3: Vec<rmcp Content> -> Vec<ContentBlock> -> RichContent
impl From<Vec<RmcpContent>> for RichContent {
    fn from(v: Vec<RmcpContent>) -> Self {
        v.into_iter()
            .map(ContentBlock::from)
            .collect::<Vec<ContentBlock>>()
            .into()
    }
}

// B4: rmcp ImageContent -> ImageContent -> ImageUrl
impl From<RmcpImageContent> for ImageUrl {
    fn from(v: RmcpImageContent) -> Self {
        ImageContent::from(v).into()
    }
}

// B5: rmcp AudioContent -> AudioContent -> InputAudio
impl From<RmcpAudioContent> for InputAudio {
    fn from(v: RmcpAudioContent) -> Self {
        AudioContent::from(v).into()
    }
}

// ── tests (authored; not run per the no-build directive) ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> Option<IndexMap<String, Value>> {
        let mut m = IndexMap::new();
        m.insert("k".to_string(), Value::String("v".to_string()));
        Some(m)
    }

    #[test]
    fn content_block_text_roundtrips_through_rmcp() {
        let cb = ContentBlock::Text(TextContent {
            text: "hi".to_string(),
            annotations: None,
            _meta: meta(),
        });
        let back: ContentBlock = RmcpContent::from(cb).into();
        match back {
            ContentBlock::Text(t) => {
                assert_eq!(t.text, "hi");
                assert_eq!(t._meta, meta());
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn image_content_roundtrips_through_rmcp() {
        let ic = ImageContent {
            data: "AAAA".to_string(),
            mime_type: "image/png".to_string(),
            annotations: None,
            _meta: meta(),
        };
        let back: ImageContent = RmcpImageContent::from(ic).into();
        assert_eq!(back.data, "AAAA");
        assert_eq!(back.mime_type, "image/png");
        assert_eq!(back._meta, meta());
    }

    #[test]
    fn audio_content_roundtrips_through_rmcp_meta_lost() {
        let ac = AudioContent {
            data: "BBBB".to_string(),
            mime_type: "audio/mpeg".to_string(),
            annotations: None,
            _meta: meta(),
        };
        let back: AudioContent = RmcpAudioContent::from(ac).into();
        assert_eq!(back.data, "BBBB");
        assert_eq!(back.mime_type, "audio/mpeg");
        // rmcp RawAudioContent carries no _meta — documented loss.
        assert_eq!(back._meta, None);
    }

    #[test]
    fn resource_contents_text_roundtrips_through_rmcp() {
        let rcu = ResourceContentsUnion::Text(TextResourceContents {
            base: ResourceContents {
                uri: "str:///x".to_string(),
                mime_type: Some("text/plain".to_string()),
                _meta: meta(),
            },
            text: "body".to_string(),
        });
        let back: ResourceContentsUnion = RmcpResourceContents::from(rcu).into();
        match back {
            ResourceContentsUnion::Text(t) => {
                assert_eq!(t.base.uri, "str:///x");
                assert_eq!(t.base.mime_type.as_deref(), Some("text/plain"));
                assert_eq!(t.text, "body");
                assert_eq!(t.base._meta, meta());
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }
}
