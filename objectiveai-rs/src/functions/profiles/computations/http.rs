use crate::{HttpClient, HttpError};
use futures::Stream;

pub async fn compute_profile_unary(
    client: &HttpClient,
    request: super::request::Request,
) -> Result<super::response::unary::FunctionProfileComputation, HttpError> {
    match request {
        super::request::Request::FunctionInline { mut body } => {
            body.base.stream = None;
            client
                .send_unary(
                    reqwest::Method::POST,
                    "functions/profiles/compute",
                    Some(body),
                )
                .await
        }
        super::request::Request::FunctionRemote { path, mut body } => {
            body.stream = None;
            client
                .send_unary(
                    reqwest::Method::POST,
                    match path.fcommit {
                        Some(fcommit) => {
                            format!(
                                "functions/{}/{}/{}/profiles/compute",
                                path.fowner, path.frepository, fcommit
                            )
                        }
                        None => format!(
                            "functions/{}/{}/profiles/compute",
                            path.fowner, path.frepository
                        ),
                    },
                    Some(body),
                )
                .await
        }
    }
}

pub async fn compute_profile_streaming(
    client: &HttpClient,
    request: super::request::Request,
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
    match request {
        super::request::Request::FunctionInline { mut body } => {
            body.base.stream = Some(true);
            Ok(futures::future::Either::Left(
                client
                    .send_streaming(
                        reqwest::Method::POST,
                        "functions/profiles/compute",
                        Some(body),
                    )
                    .await?,
            ))
        }
        super::request::Request::FunctionRemote { path, mut body } => {
            body.stream = Some(true);
            Ok(futures::future::Either::Right(
                client
                    .send_streaming(
                        reqwest::Method::POST,
                        match path.fcommit {
                            Some(fcommit) => {
                                format!(
                                    "functions/{}/{}/{}/profiles/compute",
                                    path.fowner, path.frepository, fcommit
                                )
                            }
                            None => format!(
                                "functions/{}/{}/profiles/compute",
                                path.fowner, path.frepository
                            ),
                        },
                        Some(body),
                    )
                    .await?,
            ))
        }
    }
}
