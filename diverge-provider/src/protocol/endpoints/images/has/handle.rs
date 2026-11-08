//! Answering whether an image is held, from a scope and a holder.

use diverge_sdk::shared::Identity;
use diverge_sdk::provider::endpoints::images::has::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::provider::endpoints::images::has::client::request;
use crate::protocol::image_holder::ImageHolder;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;

/// Answer the question and end the scope.
///
/// The rules:
///
/// 1. **Nowhere to look.** A provider whose capabilities keep no
///    Store and list no registry answers
///    [`Unsupported`](response::Frame::Unsupported), and the holder is
///    never asked.
/// 2. **Held.** The holder's `true` is
///    [`Held`](response::Frame::Held): the caller's own Store, or a
///    registry the provider uses that one of the image's references
///    names, has it.
/// 3. **Not found.** Its `false` is
///    [`NotFound`](response::Frame::NotFound) — an answer: the
///    provider looked and will not supply the image to this caller.
/// 4. **No answer.** Its [`Err`] is
///    [`Error`](response::Frame::Error), flattened: the absence of an
///    answer, which a caller that treated as a no would conclude an
///    image is not held when it was never told. Nothing here turns a
///    no into a failure or a failure into a no.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<H>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &Identity,
    holder: &H,
    capabilities: &Capabilities,
) where
    H: ImageHolder,
    H::Error: Into<Error>,
{
    let frame = if !capabilities.images_store && capabilities.images_registries.is_empty() {
        response::Frame::Unsupported
    } else {
        match holder.has(client_identity, &request.0.image).await {
            Ok(true) => response::Frame::Held,
            Ok(false) => response::Frame::NotFound,
            Err(error) => response::Frame::Error(error.into()),
        }
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
