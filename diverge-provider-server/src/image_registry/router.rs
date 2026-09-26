//! The HTTP: what podman sends, answered.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use dashmap::DashMap;
use diverge_sdk::provider::server::image_source::Manifest;
use serde_json::json;

use super::{Ask, Digest, Kind, Repository, verified};

/// The repositories being served, by name: what the router answers
/// from.
pub type Repositories = Arc<DashMap<String, Arc<Repository>>>;

/// The header a Distribution API registry answers `GET /v2/` with.
const API_VERSION: &str = "registry/2.0";

/// The registry's routes: `/v2/` answering that this is a registry,
/// and everything under it answered by `answer` for `GET` and
/// `HEAD`. Any other method, and any other path, is a `404`.
pub fn router(repositories: Repositories) -> Router {
    Router::new()
        .route("/v2", get(base))
        .route("/v2/", get(base))
        .route("/v2/{*rest}", any(answer))
        .with_state(repositories)
}

/// `GET /v2/`: `200`, and the API version.
async fn base() -> Response {
    ([(header::HeaderName::from_static("docker-distribution-api-version"), API_VERSION)], StatusCode::OK).into_response()
}

/// Everything under `/v2/`: the path read as an [`Ask`], the
/// repository found, and the manifest or blob answered.
async fn answer(State(repositories): State<Repositories>, method: Method, Path(rest): Path<String>) -> Response {
    let head = match method {
        Method::GET => false,
        Method::HEAD => true,
        _ => return not_found("UNSUPPORTED", "only GET and HEAD are served"),
    };
    let Some(ask) = Ask::parse(&rest) else {
        return not_found("NAME_UNKNOWN", "not a manifest or a blob of a repository");
    };
    let Some(repository) = repositories.get(&ask.repository).map(|repository| Arc::clone(&repository)) else {
        return not_found("NAME_UNKNOWN", "no such repository");
    };
    match ask.kind {
        Kind::Manifest(reference) => manifest(&repository, &reference, head).await,
        Kind::Blob(digest) => blob(&repository, &digest, head).await,
    }
}

/// A manifest by reference: the reference must be a digest, since a
/// client image is pinned by one and a tag is never pulled; the
/// manifest is answered with its media type as `Content-Type`, its
/// digest, and its length, and its bytes unless the request was a
/// `HEAD`.
async fn manifest(repository: &Repository, reference: &str, head: bool) -> Response {
    let Some(digest) = Digest::parse(reference) else {
        return not_found("MANIFEST_UNKNOWN", "manifests are served by digest");
    };
    let Some(Manifest { media_type, body }) = repository.manifest(&digest).await else {
        return not_found("MANIFEST_UNKNOWN", "the caller does not hold the manifest");
    };
    let mut response = Response::new(if head { Body::empty() } else { Body::from(body.clone()) });
    let headers = response.headers_mut();
    if let Ok(media_type) = HeaderValue::from_str(&media_type) {
        headers.insert(header::CONTENT_TYPE, media_type);
    }
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(body.len()));
    headers.insert(header::HeaderName::from_static("docker-content-digest"), digest_value(&digest));
    response
}

/// A blob by digest: a `HEAD` is the headers alone, with the length
/// where a manifest's descriptor declared it, and asks the caller
/// nothing; a `GET` is the caller's pieces streamed through
/// [`verified`], with the same headers. A `Range` header is not read:
/// the whole blob is answered with `200`, and the runtime falls back
/// to an ordinary download.
async fn blob(repository: &Repository, digest: &str, head: bool) -> Response {
    let Some(digest) = Digest::parse(digest) else {
        return not_found("BLOB_UNKNOWN", "not a digest");
    };
    let body = if head {
        Body::empty()
    } else {
        let Some(pieces) = repository.blob(&digest).await else {
            return not_found("BLOB_UNKNOWN", "the caller does not hold the blob");
        };
        Body::from_stream(verified(digest.clone(), pieces))
    };
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    if let Some(size) = repository.size(&digest) {
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from(size));
    }
    headers.insert(header::HeaderName::from_static("docker-content-digest"), digest_value(&digest));
    response
}

/// The digest as a header value; a parsed digest is always one.
fn digest_value(digest: &Digest) -> HeaderValue {
    HeaderValue::from_str(&digest.to_string()).unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// `404` with the Distribution API's error body, which is what makes
/// the runtime give up rather than retry.
fn not_found(code: &str, message: &str) -> Response {
    let body = json!({ "errors": [{ "code": code, "message": message }] });
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}
