//! What this agent makes of a message's content blocks.
//!
//! A message is MCP content — text, an image, audio, an embedded
//! resource, a link to one — and this harness takes text on stdin and images by file, `codex exec -i`; audio and a binary resource that is not an image it cannot. A block
//! it cannot take refuses the WHOLE message, at `/run` before the
//! loop starts and at `/enqueue` before the message is queued, so
//! its fate is the error and nothing is silently reduced to its
//! text. [`check`] is that judgment; [`render`] is the text and the image files a checked message becomes.

use futures_util::future;
use rmcp::model::{ContentBlock, ResourceContents};
use serde_json::Value;

/// The refusal for a block this agent cannot take.
fn unsupported(kind: &str) -> Value {
    serde_json::json!({
        "kind": "content",
        "error": format!("{kind} content is not supported by this agent"),
    })
}

/// The refusal for a message with nothing in it.
fn empty() -> Value {
    serde_json::json!({
        "kind": "content",
        "error": "a message needs content",
    })
}

/// A resource link, as the one line of text it can be.
fn link(name: &str, uri: &str) -> String {
    format!("{name} <{uri}>")
}

use std::path::{Path, PathBuf};

/// Refuse a message this agent cannot take whole: one with no block,
/// or one with audio, a binary resource that is not an image, or a
/// block this crate does not know.
pub fn check(content: &[ContentBlock]) -> Result<(), Value> {
    if content.is_empty() {
        return Err(empty());
    }
    for block in content {
        match block {
            ContentBlock::Text(_) | ContentBlock::Image(_) | ContentBlock::ResourceLink(_) => {}
            ContentBlock::Audio(_) => return Err(unsupported("audio")),
            ContentBlock::Resource(embedded) => match &embedded.resource {
                ResourceContents::TextResourceContents { .. } => {}
                ResourceContents::BlobResourceContents { mime_type, .. } => {
                    if !mime_type.as_deref().is_some_and(|mime| mime.starts_with("image/")) {
                        return Err(unsupported("binary resource"));
                    }
                }
                _ => return Err(unsupported("unknown resource")),
            },
            _ => return Err(unsupported("unknown")),
        }
    }
    Ok(())
}

/// One image of a message, written to disk for `-i`.
pub struct Image {
    /// The file's name, its extension from the image's type.
    pub name: String,
    /// The bytes, as the block carried them: base64.
    pub base64: String,
}

/// A checked message as one turn's input: the text — every text
/// block and every link, joined by blank lines — and the images, in
/// order, for the files `-i` names. A message that reaches here was
/// checked.
pub fn render(content: &[ContentBlock]) -> (String, Vec<Image>) {
    let mut texts = Vec::new();
    let mut images = Vec::new();
    for block in content {
        match block {
            ContentBlock::Text(text) => texts.push(text.text.clone()),
            ContentBlock::ResourceLink(resource) => texts.push(link(&resource.name, &resource.uri)),
            ContentBlock::Image(image) => images.push(Image {
                name: format!("{}.{}", uuid::Uuid::new_v4(), extension(&image.mime_type)),
                base64: image.data.clone(),
            }),
            ContentBlock::Resource(embedded) => match &embedded.resource {
                ResourceContents::TextResourceContents { text, .. } => texts.push(text.clone()),
                ResourceContents::BlobResourceContents { blob, mime_type, .. } => images.push(Image {
                    name: format!("{}.{}", uuid::Uuid::new_v4(), extension(mime_type.as_deref().unwrap_or(""))),
                    base64: blob.clone(),
                }),
                _ => {}
            },
            _ => {}
        }
    }
    (texts.join("\n\n"), images)
}

/// The file extension Codex reads an image's type from.
fn extension(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png",
    }
}

/// Where a turn's images are written: one directory per process,
/// under the system's temporary directory.
pub fn directory() -> PathBuf {
    std::env::temp_dir().join("diverge-codex-images")
}

/// Write a turn's images, every file beside every other, and hand
/// back their paths in the message's order; the files are the turn's,
/// removed by [`remove`] when it ends. One image that will not write
/// fails the turn; the files already written are the turn's to
/// remove all the same.
pub async fn write(images: &[Image]) -> std::io::Result<Vec<PathBuf>> {
    let directory = directory();
    tokio::fs::create_dir_all(&directory).await?;
    future::try_join_all(images.iter().map(|image| write_one(&directory, image))).await
}

/// One image to its file.
async fn write_one(directory: &Path, image: &Image) -> std::io::Result<PathBuf> {
    let bytes = base64_decode(&image.base64)?;
    let path = directory.join(&image.name);
    tokio::fs::write(&path, bytes).await?;
    Ok(path)
}

/// Remove a turn's images, every file beside every other. A file
/// that is already gone is not an error worth reporting: nothing
/// reads it again.
pub async fn remove(paths: &[PathBuf]) {
    future::join_all(paths.iter().map(tokio::fs::remove_file)).await;
}

/// Base64, decoded by hand: the standard alphabet with padding, as
/// MCP carries bytes, and no dependency for forty lines.
fn base64_decode(text: &str) -> std::io::Result<Vec<u8>> {
    fn value(byte: u8) -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some((byte - b'A') as u32),
            b'a'..=b'z' => Some((byte - b'a') as u32 + 26),
            b'0'..=b'9' => Some((byte - b'0') as u32 + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in text.bytes() {
        if byte == b'=' || byte == b'\n' || byte == b'\r' {
            continue;
        }
        let Some(value) = value(byte) else {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "the image is not base64"));
        };
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}
