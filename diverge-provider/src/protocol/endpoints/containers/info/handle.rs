//! Answering an image's info, from a scope and a deployer.

use std::fmt;

use diverge_sdk::provider::endpoints::containers::info::client::request;
use diverge_sdk::provider::endpoints::containers::info::server::response;
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::protocol::container_deployer::ContainerDeployer;
use crate::protocol::image_info::ImageInfo;

/// Answer the image's `/DIVERGE.md` and end the scope: exactly one
/// response, then the finish.
///
/// The rules, in order, over what the deployer read:
///
/// 1. The image found nowhere is `NoSource`.
/// 2. No file, or not a regular one, is `Missing`: a fact about the
///    image and not a failure.
/// 3. A file whose bytes are not UTF-8 is `Error` of kind `info`.
/// 4. A text that breaks a rule of [`Document::parse`] is `Error` of
///    kind `info`, naming the rule.
/// 5. Otherwise `Info`, the document.
/// 6. A deployer that failed is `Error`, its own failure rendered.
///
/// Rules 3 and 4 are the container proxy's own for the run's info
/// channel, so a file answers alike read either way.
pub async fn handle<D>(scope: ScopeHandle, request: request::Frame, client_identity: &Identity, deployer: &D)
where
    D: ContainerDeployer,
    D::Error: Into<Error>,
{
    let frame = match deployer.read_info(client_identity, &request.image).await {
        Ok(ImageInfo::Unavailable) => response::Frame::NoSource,
        Ok(ImageInfo::Missing) => response::Frame::Missing,
        Ok(ImageInfo::Found(bytes)) => match String::from_utf8(bytes) {
            Err(reason) => response::Frame::Error(info_error(reason)),
            Ok(text) => match Document::parse(&text) {
                Ok(document) => response::Frame::Info(document),
                Err(reason) => response::Frame::Error(info_error(reason)),
            },
        },
        Err(error) => response::Frame::Error(error.into()),
    };
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// A failure of the document's own, in the wire's one error shape:
/// kind `info`, as the proxy says it.
fn info_error(reason: impl fmt::Display) -> Error {
    Error(serde_json::json!({
        "kind": "info",
        "error": reason.to_string(),
    }))
}
