//! The image's `/DIVERGE.md`: read from the container's root at
//! every ask and answered on the server's channel.

use std::fmt;
use std::sync::Arc;

use diverge_sdk::shared::containers::info::{Document, response};
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::encode::encoded;
use crate::reply::reply;

/// Where every image puts its document: the root of its filesystem,
/// which is the root of the container's.
pub const PATH: &str = "/DIVERGE.md";

/// Answer the server's info channel with the image's document: one
/// frame, then the finish.
///
/// The rules, in order:
///
/// 1. The file is looked for at [`PATH`], a symlink followed. Nothing
///    there is `Missing`; so is anything there that is not a regular
///    file — a directory above all — since an image that puts a
///    directory at the path ships no document, and the shared read's
///    doctrine never reads a directory. Either is a fact about the
///    image and not a failure. Any other failure to look is `Error`.
/// 2. The file is read whole as UTF-8. A read that fails, or bytes
///    that are not UTF-8, is `Error`, with the reason.
/// 3. The text is parsed as [`Document::parse`] states. A text that
///    breaks one of its rules is `Error`, naming the rule.
/// 4. Otherwise `Info`, the document.
///
/// Every `Error` is of kind `info`. The program is asked nothing: the
/// file is the image's, not the program's, and the proxy reads it
/// afresh at every ask.
pub async fn info(scope: Arc<ScopeHandle>, channel: u32) {
    let frame = match tokio::fs::metadata(PATH).await {
        Err(reason) if reason.kind() == std::io::ErrorKind::NotFound => response::Frame::Missing,
        Err(reason) => response::Frame::Error(error(reason)),
        Ok(metadata) if !metadata.is_file() => response::Frame::Missing,
        Ok(_) => match tokio::fs::read_to_string(PATH).await {
            Err(reason) => response::Frame::Error(error(reason)),
            Ok(text) => match Document::parse(&text) {
                Ok(document) => response::Frame::Info(document),
                Err(reason) => response::Frame::Error(error(reason)),
            },
        },
    };
    reply(&scope, channel, encoded(&frame)).await;
}

/// A failure of the document's own, in the wire's one error shape:
/// kind `info`.
fn error(reason: impl fmt::Display) -> Error {
    Error(serde_json::json!({
        "kind": "info",
        "error": reason.to_string(),
    }))
}
