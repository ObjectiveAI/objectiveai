use crate::{HttpClient, HttpError};
use futures::Stream;

pub async fn compute_profile_unary(
    client: &HttpClient,
    mut params: super::request::FunctionProfileComputationCreateParams,
) -> Result<super::response::unary::FunctionProfileComputation, HttpError> {
    params.stream = None;
    client
        .send_unary(
            reqwest::Method::POST,
            "functions/profiles/compute",
            Some(params),
        )
        .await
}

pub async fn compute_profile_streaming(
    client: &HttpClient,
    mut params: super::request::FunctionProfileComputationCreateParams,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::FunctionProfileComputationChunk,
            HttpError,
        >,
    >
    + Send
    + 'static
    + use<>,
    HttpError,
> {
    params.stream = Some(true);
    client
        .send_streaming(
            reqwest::Method::POST,
            "functions/profiles/compute",
            Some(params),
        )
        .await
}
