//! Mechanical bridge from the SDK's `ContentBlock` family to
//! `rmcp::model::Content`. Each variant maps 1:1; `_meta` is carried
//! across via the [`sdk_meta_to_rmcp`] helper so protocol-level
//! metadata survives the projection.
//!
//! `RawAudioContent` in rmcp has no `meta` field, so audio meta is the
//! one round-trip loss; everything else preserves it.

use objectiveai_sdk::mcp::shared::ResourceContentsUnion;
use objectiveai_sdk::mcp::tool::ContentBlock;
use rmcp::model::{
    Annotated, Content, Meta, RawAudioContent, RawContent, RawResource, RawTextContent,
    ResourceContents,
};

/// Convert one SDK `ContentBlock` into one rmcp `Content`. Carries the
/// SDK's `_meta` field across to rmcp's `meta` field (rebuilt as a
/// `serde_json::Map` inside an rmcp `Meta` wrapper). Drops SDK-side
/// `annotations` (rmcp's `Annotated { annotations: None }` is what
/// `Content::text` / `Content::image` produce, and our formatter
/// doesn't carry annotations through).
pub fn into_rmcp_content(block: ContentBlock) -> Content {
    match block {
        ContentBlock::Text(t) => Annotated {
            raw: RawContent::Text(RawTextContent {
                text: t.text,
                meta: sdk_meta_to_rmcp(t._meta),
            }),
            annotations: None,
        },
        ContentBlock::Image(i) => Annotated {
            raw: RawContent::Image(rmcp::model::RawImageContent {
                data: i.data,
                mime_type: i.mime_type,
                meta: sdk_meta_to_rmcp(i._meta),
            }),
            annotations: None,
        },
        ContentBlock::Audio(a) => Annotated {
            raw: RawContent::Audio(RawAudioContent {
                data: a.data,
                mime_type: a.mime_type,
                // rmcp's `RawAudioContent` has no `meta` field, so
                // `a._meta` is intentionally dropped here.
            }),
            annotations: None,
        },
        ContentBlock::EmbeddedResource(er) => Annotated {
            raw: RawContent::Resource(rmcp::model::RawEmbeddedResource {
                resource: rcu_to_rmcp(er.resource),
                meta: sdk_meta_to_rmcp(er._meta),
            }),
            annotations: None,
        },
        ContentBlock::ResourceLink(rl) => Annotated {
            raw: RawContent::ResourceLink(RawResource {
                uri: rl.uri,
                name: rl.name,
                title: rl.title,
                description: rl.description,
                mime_type: rl.mime_type,
                size: None,
                icons: None,
                meta: sdk_meta_to_rmcp(rl._meta),
            }),
            annotations: None,
        },
    }
}

/// SDK `Option<IndexMap<String, Value>>` → rmcp `Option<Meta>`. The
/// rmcp `Meta` wraps a `serde_json::Map<String, Value>`; we rebuild it
/// preserving insertion order (the SDK's `IndexMap` and serde_json's
/// `Map` with `preserve_order` are both insertion-ordered).
fn sdk_meta_to_rmcp(
    meta: Option<indexmap::IndexMap<String, serde_json::Value>>,
) -> Option<Meta> {
    meta.map(|m| {
        let mut map = serde_json::Map::with_capacity(m.len());
        for (k, v) in m {
            map.insert(k, v);
        }
        Meta(map)
    })
}

fn rcu_to_rmcp(rcu: ResourceContentsUnion) -> ResourceContents {
    match rcu {
        ResourceContentsUnion::Text(t) => ResourceContents::TextResourceContents {
            uri: t.base.uri,
            mime_type: t.base.mime_type,
            text: t.text,
            meta: sdk_meta_to_rmcp(t.base._meta),
        },
        ResourceContentsUnion::Blob(b) => ResourceContents::BlobResourceContents {
            uri: b.base.uri,
            mime_type: b.base.mime_type,
            blob: b.blob,
            meta: sdk_meta_to_rmcp(b.base._meta),
        },
    }
}
