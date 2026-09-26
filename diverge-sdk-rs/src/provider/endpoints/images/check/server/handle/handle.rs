//! Answering an image check, from a scope and a checker.


use super::super::response;
use crate::wire::encode::{Encode, Writer};
use crate::provider::endpoints::images::check::client::request;
use crate::provider::server::image_checker::ImageChecker;
use crate::wire::server::scope_handle::ScopeHandle;
use crate::shared::error::Error;

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
/// # The request arrives decoded
///
/// [`server::handle`](crate::provider::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<C>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    checker: &C,
) where
    C: ImageChecker,
    C::Error: Into<Error>,
{
    let frame = match checker
        .check(client_identity, &request.name, &request.digest)
        .await
    {
        Ok(response) => response::Frame::Response(response),
        Err(error) => response::Frame::Error(error.into()),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
