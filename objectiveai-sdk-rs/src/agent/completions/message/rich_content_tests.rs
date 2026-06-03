//! Coverage for the `From<ContentBlock> for RichContentPart` /
//! `From<ResourceContentsUnion> for RichContentPart` mime-prefix
//! dispatch — including the `data:<mime>;base64,<payload>`
//! detection on the text path that lets `ContentBlock::Text` carry
//! inline media.

#![cfg(feature = "mcp")]

use crate::agent::completions::message::{
    File, ImageUrl, InputAudio, RichContentPart, VideoUrl,
};
use crate::mcp::shared::{
    BlobResourceContents, ResourceContents, ResourceContentsUnion,
    TextResourceContents,
};
use crate::mcp::tool::{ContentBlock, TextContent};

fn text_block(text: &str) -> ContentBlock {
    ContentBlock::Text(TextContent {
        text: text.to_string(),
        annotations: None,
        _meta: None,
    })
}

fn text_resource(uri: &str, text: &str) -> ResourceContentsUnion {
    ResourceContentsUnion::Text(TextResourceContents {
        base: ResourceContents {
            uri: uri.to_string(),
            mime_type: Some("text/plain".to_string()),
            _meta: None,
        },
        text: text.to_string(),
    })
}

fn blob_resource(uri: &str, mime: &str, blob: &str) -> ResourceContentsUnion {
    ResourceContentsUnion::Blob(BlobResourceContents {
        base: ResourceContents {
            uri: uri.to_string(),
            mime_type: Some(mime.to_string()),
            _meta: None,
        },
        blob: blob.to_string(),
    })
}

// -- ContentBlock::Text — plain text and data-URL detection ---------

#[test]
fn text_block_plain_passes_through() {
    let part: RichContentPart = text_block("hello world").into();
    match part {
        RichContentPart::Text { text } => assert_eq!(text, "hello world"),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn text_block_image_data_url_becomes_image_part() {
    let url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB";
    let part: RichContentPart = text_block(url).into();
    match part {
        RichContentPart::ImageUrl { image_url } => {
            // The whole data URL round-trips verbatim — same shape
            // `From<ResourceContentsUnion>::Blob` produces.
            assert_eq!(image_url.url, url);
            assert!(image_url.detail.is_none());
        }
        other => panic!("expected ImageUrl, got {other:?}"),
    }
}

#[test]
fn text_block_audio_data_url_becomes_input_audio_part() {
    let url = "data:audio/mpeg;base64,SUQzBAAAAAAAI1RTU0U";
    let part: RichContentPart = text_block(url).into();
    match part {
        RichContentPart::InputAudio { input_audio } => {
            assert_eq!(input_audio.data, "SUQzBAAAAAAAI1RTU0U");
            assert_eq!(input_audio.format, "audio/mpeg");
        }
        other => panic!("expected InputAudio, got {other:?}"),
    }
}

#[test]
fn text_block_video_data_url_becomes_input_video_part() {
    let url = "data:video/mp4;base64,AAAAGGZ0eXBtcDQy";
    let part: RichContentPart = text_block(url).into();
    match part {
        RichContentPart::InputVideo { video_url } => {
            assert_eq!(video_url.url, url);
        }
        other => panic!("expected InputVideo, got {other:?}"),
    }
}

#[test]
fn text_block_pdf_data_url_becomes_file_part() {
    let payload = "JVBERi0xLjQK";
    let url = format!("data:application/pdf;base64,{payload}");
    let part: RichContentPart = text_block(&url).into();
    match part {
        RichContentPart::File { file } => {
            assert_eq!(file.file_data.as_deref(), Some(payload));
            // No URI in a TextContent → no filename can be derived.
            assert!(file.filename.is_none());
            assert!(file.file_id.is_none());
            assert!(file.file_url.is_none());
        }
        other => panic!("expected File, got {other:?}"),
    }
}

#[test]
fn text_block_data_url_without_base64_marker_falls_through_to_text() {
    // `parse_data_url` requires `;base64,` — a comma-without-marker
    // form is just a plain text string from our perspective.
    let raw = "data:image/png,raw-not-base64";
    let part: RichContentPart = text_block(raw).into();
    match part {
        RichContentPart::Text { text } => assert_eq!(text, raw),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn text_block_bare_data_prefix_falls_through_to_text() {
    for malformed in ["data:", "data:image", "data:image/png", "data"] {
        let part: RichContentPart = text_block(malformed).into();
        match part {
            RichContentPart::Text { text } => assert_eq!(text, malformed),
            other => {
                panic!("expected Text for {malformed:?}, got {other:?}")
            }
        }
    }
}

// -- ResourceContentsUnion blob dispatch — same helper path ---------

#[test]
fn resource_blob_image_becomes_image_part() {
    let part: RichContentPart =
        blob_resource("file:///tmp/a.png", "image/png", "iVBOR").into();
    match part {
        RichContentPart::ImageUrl { image_url } => {
            assert_eq!(image_url.url, "data:image/png;base64,iVBOR");
        }
        other => panic!("expected ImageUrl, got {other:?}"),
    }
}

#[test]
fn resource_blob_audio_becomes_input_audio_part() {
    let part: RichContentPart =
        blob_resource("file:///tmp/a.mp3", "audio/mpeg", "SUQzBAA").into();
    match part {
        RichContentPart::InputAudio { input_audio } => {
            assert_eq!(input_audio.data, "SUQzBAA");
            assert_eq!(input_audio.format, "audio/mpeg");
        }
        other => panic!("expected InputAudio, got {other:?}"),
    }
}

#[test]
fn resource_blob_video_becomes_input_video_part() {
    let part: RichContentPart =
        blob_resource("file:///tmp/a.mp4", "video/mp4", "AAAA").into();
    match part {
        RichContentPart::InputVideo { video_url } => {
            assert_eq!(video_url.url, "data:video/mp4;base64,AAAA");
        }
        other => panic!("expected InputVideo, got {other:?}"),
    }
}

#[test]
fn resource_blob_unknown_mime_becomes_file_with_uri_filename() {
    let part: RichContentPart =
        blob_resource("file:///tmp/report.pdf", "application/pdf", "JVBERi0")
            .into();
    match part {
        RichContentPart::File { file } => {
            assert_eq!(file.file_data.as_deref(), Some("JVBERi0"));
            assert_eq!(file.filename.as_deref(), Some("report.pdf"));
        }
        other => panic!("expected File, got {other:?}"),
    }
}

#[test]
fn resource_text_becomes_text_part() {
    let part: RichContentPart =
        text_resource("file:///tmp/a.txt", "hello").into();
    match part {
        RichContentPart::Text { text } => assert_eq!(text, "hello"),
        other => panic!("expected Text, got {other:?}"),
    }
}

// Silence unused-import warnings under unusual feature combos.
#[allow(dead_code)]
fn _references() {
    let _ = ImageUrl {
        url: String::new(),
        detail: None,
    };
    let _ = InputAudio {
        data: String::new(),
        format: String::new(),
    };
    let _ = VideoUrl { url: String::new() };
    let _ = File {
        file_data: None,
        filename: None,
        file_id: None,
        file_url: None,
    };
}
