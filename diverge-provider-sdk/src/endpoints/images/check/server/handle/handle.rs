//! Answering an image check, from a scope and a checker.

use serde_json::Value;

use super::super::response;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::endpoints::images::check::client::request;
use crate::server::image_checker::ImageChecker;
use crate::server::scope_handle::ScopeHandle;
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
pub async fn handle<C>(
    mut scope: ScopeHandle,
    client_identity: &str,
    checker: &C,
) where
    C: ImageChecker,
    C::Error: Into<Error>,
{
    let frame = match request::Frame::decode(scope.request()) {
        Ok(request) => {
            match checker
                .check(client_identity, &request.name, &request.digest)
                .await
            {
                Ok(response) => response::Frame::Response(response),
                Err(error) => response::Frame::Error(error.into()),
            }
        }
        Err(error) => {
            response::Frame::Error(Error(Value::String(error.to_string())))
        }
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}
