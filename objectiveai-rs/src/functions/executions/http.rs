//! HTTP functions for function executions.

use crate::{HttpClient, HttpError};
use futures::Stream;

/// Creates a function execution (non-streaming).
///
/// Executes a function and waits for the complete response. The endpoint URL
/// is determined by the request type:
/// - Inline function + inline profile: `POST /functions`
/// - Inline function + remote profile: `POST /functions/profiles/{owner}/{repo}[/{commit}]`
/// - Remote function + inline profile: `POST /functions/{owner}/{repo}[/{commit}]`
/// - Remote function + remote profile: `POST /functions/{owner}/{repo}[/{commit}]/profiles/{owner}/{repo}[/{commit}]`
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `request` - The function execution request
///
/// # Returns
///
/// The complete function execution response.
pub async fn create_function_execution_unary(
    client: &HttpClient,
    request: super::request::Request,
) -> Result<super::response::unary::FunctionExecution, HttpError> {
    match request {
        super::request::Request::FunctionInlineProfileInline { mut body } => {
            body.base.stream = None;
            client
                .send_unary(reqwest::Method::POST, "functions", Some(body))
                .await
        }
        super::request::Request::FunctionInlineProfileRemote {
            path,
            mut body,
        } => {
            body.base.stream = None;
            client
                .send_unary(
                    reqwest::Method::POST,
                    match path.pcommit {
                        Some(pcommit) => {
                            format!(
                                "functions/profiles/{}/{}/{}",
                                path.powner, path.prepository, pcommit
                            )
                        }
                        None => format!(
                            "functions/profiles/{}/{}",
                            path.powner, path.prepository
                        ),
                    },
                    Some(body),
                )
                .await
        }
        super::request::Request::FunctionRemoteProfileInline {
            path,
            mut body,
        } => {
            body.base.stream = None;
            client
                .send_unary(
                    reqwest::Method::POST,
                    match path.fcommit {
                        Some(fcommit) => {
                            format!(
                                "functions/{}/{}/{}",
                                path.fowner, path.frepository, fcommit
                            )
                        }
                        None => {
                            format!(
                                "functions/{}/{}",
                                path.fowner, path.frepository
                            )
                        }
                    },
                    Some(body),
                )
                .await
        }
        super::request::Request::FunctionRemoteProfileRemote {
            path,
            mut body,
        } => {
            body.stream = None;
            client
                .send_unary(
                    reqwest::Method::POST,
                    match (path.fcommit, path.pcommit) {
                        (Some(fcommit), Some(pcommit)) => {
                            format!(
                                "functions/{}/{}/{}/profiles/{}/{}/{}",
                                path.fowner,
                                path.frepository,
                                fcommit,
                                path.powner,
                                path.prepository,
                                pcommit
                            )
                        }
                        (Some(fcommit), None) => format!(
                            "functions/{}/{}/{}/profiles/{}/{}",
                            path.fowner,
                            path.frepository,
                            fcommit,
                            path.powner,
                            path.prepository
                        ),
                        (None, Some(pcommit)) => format!(
                            "functions/{}/{}/profiles/{}/{}/{}",
                            path.fowner,
                            path.frepository,
                            path.powner,
                            path.prepository,
                            pcommit
                        ),
                        (None, None) => format!(
                            "functions/{}/{}/profiles/{}/{}",
                            path.fowner,
                            path.frepository,
                            path.powner,
                            path.prepository
                        ),
                    },
                    Some(body),
                )
                .await
        }
    }
}

/// Creates a streaming function execution.
///
/// Executes a function and returns a stream of response chunks as they arrive
/// via Server-Sent Events. The endpoint URL is determined by the request type
/// (see [`create_function_execution_unary`] for URL patterns).
///
/// # Arguments
///
/// * `client` - The HTTP client to use
/// * `request` - The function execution request
///
/// # Returns
///
/// A stream of function execution chunks.
pub async fn create_function_execution_streaming(
    client: &HttpClient,
    request: super::request::Request,
) -> Result<
    impl Stream<
        Item = Result<
            super::response::streaming::FunctionExecutionChunk,
            HttpError,
        >,
    >
    + Send
    + 'static
    + use<>,
    HttpError,
> {
    match request {
        super::request::Request::FunctionInlineProfileInline { mut body } => {
            body.base.stream = Some(true);
            Ok(futures::future::Either::Left(
                futures::future::Either::Left(
                    client
                        .send_streaming(
                            reqwest::Method::POST,
                            "functions",
                            Some(body),
                        )
                        .await?,
                ),
            ))
        }
        super::request::Request::FunctionInlineProfileRemote {
            path,
            mut body,
        } => {
            body.base.stream = Some(true);
            Ok(futures::future::Either::Left(
                futures::future::Either::Right(
                    client
                        .send_streaming(
                            reqwest::Method::POST,
                            match path.pcommit {
                                Some(pcommit) => {
                                    format!(
                                        "functions/profiles/{}/{}/{}",
                                        path.powner, path.prepository, pcommit
                                    )
                                }
                                None => format!(
                                    "functions/profiles/{}/{}",
                                    path.powner, path.prepository
                                ),
                            },
                            Some(body),
                        )
                        .await?,
                ),
            ))
        }
        super::request::Request::FunctionRemoteProfileInline {
            path,
            mut body,
        } => {
            body.base.stream = Some(true);
            Ok(futures::future::Either::Right(
                futures::future::Either::Left(
                    client
                        .send_streaming(
                            reqwest::Method::POST,
                            match path.fcommit {
                                Some(fcommit) => {
                                    format!(
                                        "functions/{}/{}/{}",
                                        path.fowner, path.frepository, fcommit
                                    )
                                }
                                None => {
                                    format!(
                                        "functions/{}/{}",
                                        path.fowner, path.frepository
                                    )
                                }
                            },
                            Some(body),
                        )
                        .await?,
                ),
            ))
        }
        super::request::Request::FunctionRemoteProfileRemote {
            path,
            mut body,
        } => {
            body.stream = Some(true);
            Ok(futures::future::Either::Right(
                futures::future::Either::Right(
                    client
                        .send_streaming(
                            reqwest::Method::POST,
                            match (path.fcommit, path.pcommit) {
                                (Some(fcommit), Some(pcommit)) => {
                                    format!(
                                        "functions/{}/{}/{}/profiles/{}/{}/{}",
                                        path.fowner,
                                        path.frepository,
                                        fcommit,
                                        path.powner,
                                        path.prepository,
                                        pcommit
                                    )
                                }
                                (Some(fcommit), None) => format!(
                                    "functions/{}/{}/{}/profiles/{}/{}",
                                    path.fowner,
                                    path.frepository,
                                    fcommit,
                                    path.powner,
                                    path.prepository
                                ),
                                (None, Some(pcommit)) => format!(
                                    "functions/{}/{}/profiles/{}/{}/{}",
                                    path.fowner,
                                    path.frepository,
                                    path.powner,
                                    path.prepository,
                                    pcommit
                                ),
                                (None, None) => format!(
                                    "functions/{}/{}/profiles/{}/{}",
                                    path.fowner,
                                    path.frepository,
                                    path.powner,
                                    path.prepository
                                ),
                            },
                            Some(body),
                        )
                        .await?,
                ),
            ))
        }
    }
}
