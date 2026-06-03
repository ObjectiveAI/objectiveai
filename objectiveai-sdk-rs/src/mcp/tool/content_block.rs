//! MCP content block enum.
//!
//! A content block is the union of all content types that can appear in
//! prompts, tool results, and sampling messages.
//!
//! # Round-trip with [`RichContentPart`]
//!
//! `From<RichContentPart> for ContentBlock` always produces one of
//! [`ContentBlock::Text`], [`ContentBlock::Image`], or
//! [`ContentBlock::Audio`] — never `EmbeddedResource` or
//! `ResourceLink`. Variants that have no native MCP carrier
//! (`InputVideo`, `VideoUrl`, `File`) and remote-URL `ImageUrl`s
//! land as `Text` blocks; the original variant is encoded in
//! `_meta` markers so the reverse `From<ContentBlock>` arm rebuilds
//! the identical [`RichContentPart`].
//!
//! Round-trip property: for every `RichContentPart` value `p`,
//! `RichContentPart::from(ContentBlock::from(p.clone())) == p` —
//! with two documented exceptions:
//!
//! 1. **`File` multi-field collapse.** When two or more of
//!    `file_data`, `file_url`, `file_id` are set on the same
//!    `File`, the forward conversion picks one by precedence
//!    (`file_data` > `file_url` > `file_id`) and the others are
//!    dropped. `filename` rides through losslessly via the
//!    `objectiveai/filename` meta marker.
//! 2. **`RichContentPart::Text` containing a base64 data URL.**
//!    `RichContentPart::Text { text: "data:image/png;base64,..." }`
//!    forward-converts to `ContentBlock::Text(t)` with the same
//!    body and no `kind` marker; the reverse arm spots the
//!    data-URL shape and returns a media variant (`ImageUrl`,
//!    `InputAudio`, etc.) rather than `Text`. This is intentional —
//!    a Text payload that happens to be a well-formed data URL is
//!    treated as media on every other entry point (the
//!    `From<TextContent>` arm in the SDK does the same thing), and
//!    splitting the behaviour here would be more surprising than
//!    the round-trip loss.
//!
//! ## `_meta` markers
//!
//! Three keys, all namespaced under `objectiveai/`:
//!
//! - **`objectiveai/kind`** (Text carrier only) — enum string,
//!   tells the reverse arm the Text block is the encoded form of a
//!   non-Text variant. Values:
//!   - `"image_url_remote"`: body is a remote URL for an
//!     [`RichContentPart::ImageUrl`].
//!   - `"input_video_remote"`: body is a remote URL for an
//!     [`RichContentPart::InputVideo`].
//!   - `"video_url"`: body is a URL (data or remote) for a
//!     [`RichContentPart::VideoUrl`] (overrides the default
//!     `data:video/*` → `InputVideo` heuristic).
//!   - `"file_url"`: body is a remote URL for a
//!     [`RichContentPart::File`] whose primary field is `file_url`.
//!   - `"file_id"`: body is an opaque ID for a
//!     [`RichContentPart::File`] whose primary field is `file_id`.
//! - **`objectiveai/image_detail`** (Image carrier, or Text with
//!   `kind: "image_url_remote"`) — preserves
//!   [`ImageUrl::detail`](crate::agent::completions::message::ImageUrl::detail).
//! - **`objectiveai/filename`** (any carrier representing a
//!   [`RichContentPart::File`]) — preserves
//!   [`File::filename`](crate::agent::completions::message::File::filename).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `_meta` key namespacing all round-trip markers, so we don't
/// collide with any official MCP metadata convention.
const META_KIND: &str = "objectiveai/kind";
const META_IMAGE_DETAIL: &str = "objectiveai/image_detail";
const META_FILENAME: &str = "objectiveai/filename";

/// `objectiveai/kind` enum tag values.
const KIND_IMAGE_URL_REMOTE: &str = "image_url_remote";
const KIND_INPUT_VIDEO_REMOTE: &str = "input_video_remote";
const KIND_VIDEO_URL: &str = "video_url";
const KIND_FILE_URL: &str = "file_url";
const KIND_FILE_ID: &str = "file_id";

/// A content block that can be used in prompts and tool results.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
#[schemars(rename = "mcp.tool.ContentBlock")]
pub enum ContentBlock {
    /// Text content.
    #[serde(rename = "text")]
    #[schemars(title = "Text")]
    Text(super::TextContent),
    /// Image content (base64-encoded).
    #[serde(rename = "image")]
    #[schemars(title = "Image")]
    Image(super::ImageContent),
    /// Audio content (base64-encoded).
    #[serde(rename = "audio")]
    #[schemars(title = "Audio")]
    Audio(super::AudioContent),
    /// A resource link.
    #[serde(rename = "resource_link")]
    #[schemars(title = "ResourceLink")]
    ResourceLink(super::ResourceLink),
    /// An embedded resource.
    #[serde(rename = "resource")]
    #[schemars(title = "EmbeddedResource")]
    EmbeddedResource(super::EmbeddedResource),
}

/// Convert a single `RichContentPart` into a `ContentBlock`.
///
/// Forward path produces only `Text`, `Image`, and `Audio` carriers
/// — never `EmbeddedResource` or `ResourceLink`. See the
/// module-level docs for the round-trip property and the `_meta`
/// marker catalogue.
///
/// Upstream-specific converters (`claude_agent_sdk`, `codex_sdk`)
/// use stricter `TryFrom` impls that reject unsupported parts. This
/// `From` is the generic, round-trip-preserving path used by
/// `agent/completions/notify` and the MCP tool-response formatter.
impl From<crate::agent::completions::message::RichContentPart>
    for ContentBlock
{
    fn from(part: crate::agent::completions::message::RichContentPart) -> Self {
        use crate::agent::completions::message::{
            File as RcpFile, RichContentPart,
        };
        match part {
            RichContentPart::Text { text } => {
                ContentBlock::Text(super::TextContent {
                    text,
                    annotations: None,
                    _meta: None,
                })
            }
            RichContentPart::ImageUrl { image_url } => {
                // Serialize detail (an enum) once via serde_json so
                // we hand markers a Value::String("auto"|"low"|"high"),
                // not the typed enum literal.
                let detail_value = image_url
                    .detail
                    .as_ref()
                    .and_then(|d| serde_json::to_value(d).ok());
                match super::ImageContent::try_from(image_url) {
                    Ok(mut ic) => {
                        // Data-URL path: lossless Image carrier.
                        // Detail (when present) rides in _meta.
                        if let Some(v) = detail_value {
                            let mut m = indexmap::IndexMap::new();
                            m.insert(META_IMAGE_DETAIL.to_string(), v);
                            ic._meta = Some(m);
                        }
                        ContentBlock::Image(ic)
                    }
                    Err(err) => {
                        // Remote URL: stash on Text with kind marker
                        // so reverse can rebuild ImageUrl.
                        let mut meta = indexmap::IndexMap::new();
                        meta.insert(
                            META_KIND.to_string(),
                            serde_json::Value::String(
                                KIND_IMAGE_URL_REMOTE.to_string(),
                            ),
                        );
                        if let Some(v) = detail_value {
                            meta.insert(META_IMAGE_DETAIL.to_string(), v);
                        }
                        ContentBlock::Text(super::TextContent {
                            text: err.url,
                            annotations: None,
                            _meta: Some(meta),
                        })
                    }
                }
            }
            RichContentPart::InputAudio { input_audio } => {
                ContentBlock::Audio(input_audio.into())
            }
            RichContentPart::InputVideo { video_url } => {
                // Data-URL InputVideo round-trips via the default
                // reverse heuristic (parse_data_url → video/* mime
                // → InputVideo), so no marker. Remote URL needs the
                // marker to tell the reverse it's an InputVideo.
                if crate::data_url::parse_data_url(&video_url.url).is_some() {
                    ContentBlock::Text(super::TextContent {
                        text: video_url.url,
                        annotations: None,
                        _meta: None,
                    })
                } else {
                    ContentBlock::Text(super::TextContent {
                        text: video_url.url,
                        annotations: None,
                        _meta: Some(single_meta(
                            META_KIND,
                            KIND_INPUT_VIDEO_REMOTE.to_string(),
                        )),
                    })
                }
            }
            RichContentPart::VideoUrl { video_url } => {
                // Both data-URL and remote URL need the marker —
                // without it the reverse defaults to InputVideo.
                ContentBlock::Text(super::TextContent {
                    text: video_url.url,
                    annotations: None,
                    _meta: Some(single_meta(
                        META_KIND,
                        KIND_VIDEO_URL.to_string(),
                    )),
                })
            }
            RichContentPart::File { file } => file_to_block(file),
        }
    }
}

/// Direct conversion from a typed `ImageUrl` to a `ContentBlock`.
/// Same body as the `RichContentPart::ImageUrl` arm of
/// [`From<RichContentPart> for ContentBlock`] — kept independent so
/// per-leaf `CommandResponse::into_mcp` impls (whose `Response` is
/// already an `ImageUrl`) can call `image_url.into()` without first
/// wrapping in `RichContentPart`.
impl From<crate::agent::completions::message::ImageUrl> for ContentBlock {
    fn from(image_url: crate::agent::completions::message::ImageUrl) -> Self {
        // Serialize detail (an enum) once via serde_json so
        // we hand markers a Value::String("auto"|"low"|"high"),
        // not the typed enum literal.
        let detail_value = image_url
            .detail
            .as_ref()
            .and_then(|d| serde_json::to_value(d).ok());
        match super::ImageContent::try_from(image_url) {
            Ok(mut ic) => {
                // Data-URL path: lossless Image carrier.
                // Detail (when present) rides in _meta.
                if let Some(v) = detail_value {
                    let mut m = indexmap::IndexMap::new();
                    m.insert(META_IMAGE_DETAIL.to_string(), v);
                    ic._meta = Some(m);
                }
                ContentBlock::Image(ic)
            }
            Err(err) => {
                // Remote URL: stash on Text with kind marker
                // so reverse can rebuild ImageUrl.
                let mut meta = indexmap::IndexMap::new();
                meta.insert(
                    META_KIND.to_string(),
                    serde_json::Value::String(
                        KIND_IMAGE_URL_REMOTE.to_string(),
                    ),
                );
                if let Some(v) = detail_value {
                    meta.insert(META_IMAGE_DETAIL.to_string(), v);
                }
                ContentBlock::Text(super::TextContent {
                    text: err.url,
                    annotations: None,
                    _meta: Some(meta),
                })
            }
        }
    }
}

/// Direct conversion from a typed `InputAudio` to a `ContentBlock`.
/// Same body as the `RichContentPart::InputAudio` arm of
/// [`From<RichContentPart> for ContentBlock`].
impl From<crate::agent::completions::message::InputAudio> for ContentBlock {
    fn from(input_audio: crate::agent::completions::message::InputAudio) -> Self {
        ContentBlock::Audio(input_audio.into())
    }
}

/// Direct conversion from a typed `VideoUrl` to a `ContentBlock`.
/// Same body as the `RichContentPart::InputVideo` arm of
/// [`From<RichContentPart> for ContentBlock`] (not the `VideoUrl`
/// arm): data-URL videos round-trip via the default reverse
/// heuristic (parse_data_url → video/* mime → InputVideo), so no
/// marker. Remote URLs get `META_KIND = "input_video_remote"` so the
/// reverse rebuilds an `InputVideo`.
impl From<crate::agent::completions::message::VideoUrl> for ContentBlock {
    fn from(video_url: crate::agent::completions::message::VideoUrl) -> Self {
        if crate::data_url::parse_data_url(&video_url.url).is_some() {
            ContentBlock::Text(super::TextContent {
                text: video_url.url,
                annotations: None,
                _meta: None,
            })
        } else {
            ContentBlock::Text(super::TextContent {
                text: video_url.url,
                annotations: None,
                _meta: Some(single_meta(
                    META_KIND,
                    KIND_INPUT_VIDEO_REMOTE.to_string(),
                )),
            })
        }
    }
}

/// Direct conversion from a typed `File` to a `ContentBlock`. Same
/// body as the private [`file_to_block`] helper — kept independent
/// so per-leaf `CommandResponse::into_mcp` impls (whose `Response`
/// is already a `File`) can call `file.into()` without first
/// wrapping in `RichContentPart`. Multi-field collapse:
/// `file_data` > `file_url` > `file_id` by precedence. Lower-priority
/// fields are dropped; `filename` rides through via `_meta`.
impl From<crate::agent::completions::message::File> for ContentBlock {
    fn from(file: crate::agent::completions::message::File) -> Self {
        let filename = file.filename.clone();
        if let Some(blob) = file.file_data {
            // Encode as a Text(data:application/octet-stream;base64,...)
            // — the heuristic reverse decodes it into a File. Filename
            // rides in _meta.
            let body = format!("data:application/octet-stream;base64,{blob}");
            let meta = filename.map(|n| single_meta(META_FILENAME, n));
            ContentBlock::Text(super::TextContent {
                text: body,
                annotations: None,
                _meta: meta,
            })
        } else if let Some(url) = file.file_url {
            let mut m = single_meta(META_KIND, KIND_FILE_URL.to_string());
            if let Some(n) = filename {
                m.insert(META_FILENAME.to_string(), serde_json::Value::String(n));
            }
            ContentBlock::Text(super::TextContent {
                text: url,
                annotations: None,
                _meta: Some(m),
            })
        } else if let Some(id) = file.file_id {
            let mut m = single_meta(META_KIND, KIND_FILE_ID.to_string());
            if let Some(n) = filename {
                m.insert(META_FILENAME.to_string(), serde_json::Value::String(n));
            }
            ContentBlock::Text(super::TextContent {
                text: id,
                annotations: None,
                _meta: Some(m),
            })
        } else {
            // Empty File: nothing to encode. Produce a Text("") carrier
            // with no markers. Reverse will land it as a Text part —
            // which is a minor round-trip loss for the (unusual)
            // empty-File case. Document this in the round-trip caveats.
            ContentBlock::Text(super::TextContent {
                text: String::new(),
                annotations: None,
                _meta: None,
            })
        }
    }
}

/// Build a `ContentBlock` for a `File` part. Multi-field collapse:
/// `file_data` > `file_url` > `file_id` by precedence. Lower-priority
/// fields are dropped; `filename` rides through via `_meta`.
fn file_to_block(
    file: crate::agent::completions::message::File,
) -> ContentBlock {
    let filename = file.filename.clone();
    if let Some(blob) = file.file_data {
        // Encode as a Text(data:application/octet-stream;base64,...)
        // — the heuristic reverse decodes it into a File. Filename
        // rides in _meta.
        let body = format!("data:application/octet-stream;base64,{blob}");
        let meta = filename.map(|n| single_meta(META_FILENAME, n));
        ContentBlock::Text(super::TextContent {
            text: body,
            annotations: None,
            _meta: meta,
        })
    } else if let Some(url) = file.file_url {
        let mut m = single_meta(META_KIND, KIND_FILE_URL.to_string());
        if let Some(n) = filename {
            m.insert(META_FILENAME.to_string(), serde_json::Value::String(n));
        }
        ContentBlock::Text(super::TextContent {
            text: url,
            annotations: None,
            _meta: Some(m),
        })
    } else if let Some(id) = file.file_id {
        let mut m = single_meta(META_KIND, KIND_FILE_ID.to_string());
        if let Some(n) = filename {
            m.insert(META_FILENAME.to_string(), serde_json::Value::String(n));
        }
        ContentBlock::Text(super::TextContent {
            text: id,
            annotations: None,
            _meta: Some(m),
        })
    } else {
        // Empty File: nothing to encode. Produce a Text("") carrier
        // with no markers. Reverse will land it as a Text part —
        // which is a minor round-trip loss for the (unusual)
        // empty-File case. Document this in the round-trip caveats.
        ContentBlock::Text(super::TextContent {
            text: String::new(),
            annotations: None,
            _meta: None,
        })
    }
}

/// Build a single-entry `_meta` map.
fn single_meta(
    key: &str,
    value: String,
) -> indexmap::IndexMap<String, serde_json::Value> {
    let mut m = indexmap::IndexMap::new();
    m.insert(key.to_string(), serde_json::Value::String(value));
    m
}

/// Flatten a `RichContent` into the MCP `Vec<ContentBlock>` shape used
/// by `POST /notify` and tool results. `RichContent::Text` yields a
/// single text block; `RichContent::Parts` delegates per-element to
/// [`From<RichContentPart>`].
impl From<crate::agent::completions::message::RichContent>
    for Vec<ContentBlock>
{
    fn from(content: crate::agent::completions::message::RichContent) -> Self {
        use crate::agent::completions::message::RichContent;
        match content {
            RichContent::Text(text) => {
                vec![ContentBlock::Text(super::TextContent {
                    text,
                    annotations: None,
                    _meta: None,
                })]
            }
            RichContent::Parts(parts) => {
                parts.into_iter().map(Into::into).collect()
            }
        }
    }
}

#[cfg(test)]
mod round_trip_tests {
    use super::*;
    use crate::agent::completions::message::{
        File, ImageUrl, ImageUrlDetail, InputAudio, RichContentPart, VideoUrl,
    };

    /// Normalize a RichContentPart for round-trip comparison: clear
    /// `File::filename` (which round-trips via `_meta` but the test
    /// candidates set it to verify the marker is honored) — used
    /// only when the test expects a documented loss.
    fn norm(part: &mut RichContentPart) {
        // Currently nothing to clear universally; filename round-
        // trips via the `objectiveai/filename` marker. Kept as a
        // hook for future documented losses.
        let _ = part;
    }

    fn assert_round_trips(part: RichContentPart) {
        let mut expected = part.clone();
        norm(&mut expected);
        let block: ContentBlock = part.into();
        let mut back: RichContentPart = block.into();
        norm(&mut back);
        assert_eq!(
            expected, back,
            "round-trip mismatch: expected {expected:?}, got {back:?}"
        );
    }

    #[test]
    fn rt_text_plain() {
        assert_round_trips(RichContentPart::Text {
            text: "hello world".into(),
        });
    }

    #[test]
    fn rt_image_url_data_url_no_detail() {
        assert_round_trips(RichContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "data:image/png;base64,iVBORw0KGgo".into(),
                detail: None,
            },
        });
    }

    #[test]
    fn rt_image_url_data_url_with_detail_high() {
        assert_round_trips(RichContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "data:image/png;base64,iVBORw0KGgo".into(),
                detail: Some(ImageUrlDetail::High),
            },
        });
    }

    #[test]
    fn rt_image_url_data_url_with_detail_low() {
        assert_round_trips(RichContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "data:image/jpeg;base64,/9j/4AAQ".into(),
                detail: Some(ImageUrlDetail::Low),
            },
        });
    }

    #[test]
    fn rt_image_url_remote_url_no_detail() {
        assert_round_trips(RichContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "https://example.com/a.png".into(),
                detail: None,
            },
        });
    }

    #[test]
    fn rt_image_url_remote_url_with_detail() {
        assert_round_trips(RichContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "https://example.com/a.png".into(),
                detail: Some(ImageUrlDetail::Auto),
            },
        });
    }

    #[test]
    fn rt_input_audio() {
        assert_round_trips(RichContentPart::InputAudio {
            input_audio: InputAudio {
                data: "SUQzBAA".into(),
                format: "audio/mpeg".into(),
            },
        });
    }

    #[test]
    fn rt_input_video_data_url() {
        assert_round_trips(RichContentPart::InputVideo {
            video_url: VideoUrl {
                url: "data:video/mp4;base64,AAAA".into(),
            },
        });
    }

    #[test]
    fn rt_input_video_remote_url() {
        assert_round_trips(RichContentPart::InputVideo {
            video_url: VideoUrl {
                url: "https://example.com/v.mp4".into(),
            },
        });
    }

    #[test]
    fn rt_video_url_data_url() {
        assert_round_trips(RichContentPart::VideoUrl {
            video_url: VideoUrl {
                url: "data:video/webm;base64,GkXfo".into(),
            },
        });
    }

    #[test]
    fn rt_video_url_remote_url() {
        assert_round_trips(RichContentPart::VideoUrl {
            video_url: VideoUrl {
                url: "https://example.com/clip.webm".into(),
            },
        });
    }

    #[test]
    fn rt_file_with_file_data_no_filename() {
        assert_round_trips(RichContentPart::File {
            file: File {
                file_data: Some("JVBERi0".into()),
                filename: None,
                file_id: None,
                file_url: None,
            },
        });
    }

    #[test]
    fn rt_file_with_file_data_and_filename() {
        assert_round_trips(RichContentPart::File {
            file: File {
                file_data: Some("JVBERi0".into()),
                filename: Some("report.pdf".into()),
                file_id: None,
                file_url: None,
            },
        });
    }

    #[test]
    fn rt_file_with_file_url() {
        assert_round_trips(RichContentPart::File {
            file: File {
                file_data: None,
                filename: Some("remote.bin".into()),
                file_id: None,
                file_url: Some("https://example.com/x.bin".into()),
            },
        });
    }

    #[test]
    fn rt_file_with_file_id() {
        assert_round_trips(RichContentPart::File {
            file: File {
                file_data: None,
                filename: Some("upstream-name.txt".into()),
                file_id: Some("file-abc123".into()),
                file_url: None,
            },
        });
    }

    /// Documented round-trip exception (case 1): multi-field File
    /// collapses to the highest-precedence field (file_data >
    /// file_url > file_id). The reverse only recovers the primary
    /// field plus `filename`.
    #[test]
    fn rt_file_multifield_collapses_to_file_data() {
        let input = RichContentPart::File {
            file: File {
                file_data: Some("JVBERi0".into()),
                filename: Some("multi.bin".into()),
                file_id: Some("ignored-id".into()),
                file_url: Some("https://example.com/ignored".into()),
            },
        };
        let block: ContentBlock = input.into();
        let back: RichContentPart = block.into();
        let expected = RichContentPart::File {
            file: File {
                file_data: Some("JVBERi0".into()),
                filename: Some("multi.bin".into()),
                file_id: None,
                file_url: None,
            },
        };
        assert_eq!(back, expected);
    }

    /// Documented round-trip exception (case 2): a Text part whose
    /// body is a well-formed data URL forward-converts to Text(t)
    /// with no marker; the reverse spots the data URL and returns
    /// a media variant. Lock the actual behaviour in here so a
    /// future regression that "fixes" this case is caught.
    #[test]
    fn rt_text_containing_data_url_decodes_to_media() {
        let input = RichContentPart::Text {
            text: "data:image/png;base64,iVBORw0KGgo".into(),
        };
        let block: ContentBlock = input.into();
        let back: RichContentPart = block.into();
        assert!(
            matches!(back, RichContentPart::ImageUrl { .. }),
            "expected media, got {back:?}"
        );
    }
}
