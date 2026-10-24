//! Answering an image check, from a scope and a checker.


use diverge_sdk::shared::Identity;
use diverge_sdk::provider::endpoints::images::check::server::response;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::provider::endpoints::images::check::client::request;
use crate::protocol::image_checker::ImageChecker;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;

/// Answer the check and end the scope.
///
/// # A no and a failure are different frames
///
/// [`Unavailable`](response::Unavailable) travels as a
/// [`Response`](response::Frame::Response), because it is an answer:
/// the provider looked and will not supply the image.
/// [`Error`](response::Frame::Error) is the absence of an answer, and a
/// caller that treated them alike would conclude an image is
/// unavailable when it was never told.
///
/// So a checker's [`Err`] becomes the second and its
/// [`Ok`] becomes the first, whichever way the `Ok` went. Nothing here
/// turns a no into a failure or a failure into a no.
///
/// # No source is answered here
///
/// The sources asked are first narrowed to the ones the provider's
/// capabilities have: `client` only with an image store,
/// `registries` only with a registry listed. A request that leaves
/// none — neither asked, or only sources this provider has not — is
/// [`NoSource`](response::Frame::NoSource), and the checker is never
/// asked: the question was empty, for this provider, and the
/// checker's contract is that at least one source it has is set.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::protocol::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<C>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &Identity,
    checker: &C,
    capabilities: &Capabilities,
) where
    C: ImageChecker,
    C::Error: Into<Error>,
{
    let sources = request::Sources {
        client: request.sources.client && capabilities.images_store,
        registries: request.sources.registries && !capabilities.images_registries.is_empty(),
    };
    let frame = if !sources.any() {
        response::Frame::NoSource
    } else {
        match checker.check(client_identity, &request.image, &sources).await {
            Ok(response) => response::Frame::Response(response),
            Err(error) => response::Frame::Error(error.into()),
        }
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
