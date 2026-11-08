//! The HTTP: what podman sends, answered.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use dashmap::DashMap;
use crate::protocol::image_source::Manifest;
use serde_json::json;

use super::{Ask, Digest, Kind, Repository, verified};
use crate::host::image_manager::Store;

/// The tokens minted and not revoked, each with its repository.
pub type Repositories = Arc<DashMap<String, Arc<Repository>>>;

/// What the router answers from: the tokens, and the Store every one
/// of them is served from.
#[derive(Debug, Clone)]
pub struct Registry {
    /// The tokens, minted and revoked by the deployer.
    pub repositories: Repositories,
    /// The Store.
    pub store: Arc<Store>,
}

/// The header a Distribution API registry answers `GET /v2/` with.
const API_VERSION: &str = "registry/2.0";

/// The registry's routes: `/v2/` answering that this is a registry,
/// and everything under it answered by `answer` for `GET` and
/// `HEAD`. Any other method, and any other path, is a `404`.
pub fn router(registry: Registry) -> Router {
    Router::new()
        .route("/v2", get(base))
        .route("/v2/", get(base))
        .route("/v2/{*rest}", any(answer))
        .with_state(registry)
}

/// `GET /v2/`: `200`, and the API version.
async fn base() -> Response {
    ([(header::HeaderName::from_static("docker-distribution-api-version"), API_VERSION)], StatusCode::OK).into_response()
}

/// Everything under `/v2/`: the path read as an [`Ask`], the token
/// found among those minted and not revoked — anything else is `404`
/// — and the manifest or blob answered from its origin.
async fn answer(State(registry): State<Registry>, method: Method, Path(rest): Path<String>) -> Response {
    let head = match method {
        Method::GET => false,
        Method::HEAD => true,
        _ => return not_found("UNSUPPORTED", "only GET and HEAD are served"),
    };
    let Some(ask) = Ask::parse(&rest) else {
        return not_found("NAME_UNKNOWN", "not a manifest or a blob of a repository");
    };
    let Some(repository) = registry.repositories.get(&ask.repository).map(|repository| Arc::clone(&repository)) else {
        return not_found("NAME_UNKNOWN", "no such repository");
    };
    match ask.kind {
        Kind::Manifest(reference) => manifest(&registry.store, &repository, &reference, head).await,
        Kind::Blob(digest) => blob(&registry.store, &repository, &digest, head).await,
    }
}

/// A manifest by reference: the reference must be a digest, since an
/// image is pulled by one and a tag never is; the manifest is answered
/// as [`manifest_found`] answers it.
async fn manifest(store: &Arc<Store>, repository: &Repository, reference: &str, head: bool) -> Response {
    let Some(digest) = Digest::parse(reference) else {
        return not_found("MANIFEST_UNKNOWN", "manifests are served by digest");
    };
    let Some(found) = repository.manifest(store, &digest).await else {
        return not_found("MANIFEST_UNKNOWN", "the manifest cannot be had");
    };
    manifest_found(&digest, found, head)
}

/// A blob by digest, from the Store once the repository has it there:
/// its length always stated, and a `GET` the file's pieces streamed
/// through [`verified`], so a file that is not its digest's is never
/// completed.
async fn blob(store: &Arc<Store>, repository: &Repository, digest: &str, head: bool) -> Response {
    let Some(digest) = Digest::parse(digest) else {
        return not_found("BLOB_UNKNOWN", "not a digest");
    };
    let Some(size) = repository.blob(store, &digest).await else {
        return not_found("BLOB_UNKNOWN", "the blob cannot be had");
    };
    let body = if head {
        Body::empty()
    } else {
        let Some(pieces) = store.pieces(&digest).await else {
            return not_found("BLOB_UNKNOWN", "the blob cannot be had");
        };
        Body::from_stream(verified(digest.clone(), pieces))
    };
    blob_found(&digest, Some(size), body)
}

/// A manifest found, answered: its media type as `Content-Type`, its
/// digest, and its length, and its bytes unless the request was a
/// `HEAD`.
fn manifest_found(digest: &Digest, found: Manifest, head: bool) -> Response {
    let Manifest { media_type, body } = found;
    let mut response = Response::new(if head { Body::empty() } else { Body::from(body.clone()) });
    let headers = response.headers_mut();
    if let Ok(media_type) = HeaderValue::from_str(&media_type) {
        headers.insert(header::CONTENT_TYPE, media_type);
    }
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(body.len()));
    headers.insert(header::HeaderName::from_static("docker-content-digest"), digest_value(digest));
    response
}

/// A blob answered with `body`: as an octet stream, with its length
/// where `size` is known and its digest. A `Range` header is not
/// read: the whole blob is answered with `200`, and the runtime falls
/// back to an ordinary download.
fn blob_found(digest: &Digest, size: Option<u64>, body: Body) -> Response {
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    if let Some(size) = size {
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from(size));
    }
    headers.insert(header::HeaderName::from_static("docker-content-digest"), digest_value(digest));
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
