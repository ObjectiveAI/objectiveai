//! The daemon's `GET /plugins/{owner}/{name}/{version}/viewer` route:
//! build a plugin's viewer extension and stream the finished artifact
//! back as tar.gz — the build service behind viewer installs
//! (essential for clients that cannot run a JS toolchain themselves,
//! e.g. mobile viewers).
//!
//! The daemon does not build anything: it DISPATCHES. A laboratory
//! host builds the PLUGIN'S OWN viewer Containerfile — the same
//! `podman build` it already runs for the plugin's MCP image — which
//! is why this machine needs no node, no pnpm, no esbuild, and why an
//! author can reproduce a build with one `podman build`. (The one
//! exception is `development viewer set` — running the viewer APP from
//! source is a developer-machine feature and shells out to pnpm.) Host
//! selection is the ordinary laboratory load balancer (a uniformly
//! random connected host), the same one every ephemeral create rides.
//!
//! Flow: validate coordinates → `BuildCreate` on a random host, which
//! fetches the tag, builds the image, copies the declared output out
//! of it, and parks the archive → drain it with
//! `BuildRead` chunks straight into the response body. The archive is
//! NEVER buffered whole at either end. Nothing is written to the body
//! until the build SUCCEEDED, so a failed build can never look like a
//! truncated download; a dropped connection mid-drain yields a
//! truncated (invalid) gzip and nothing lands client-side.
//!
//! No lockfile, no cache: concurrent identical requests merely
//! duplicate work (a build cache can come later).

use axum::response::IntoResponse;
use objectiveai_sdk::laboratories::daemon::{
    BuildCreateRequest, BuildCreated, JsonRpcResult, RequestPayload, ResponsePayload,
    TransferIdRequest, BUILD_TAG_NOT_FOUND_CODE,
};

/// Path-meaningful characters are rejected outright in wire-supplied
/// identity segments.
fn safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains('/')
        && !segment.contains('\\')
        && segment != "."
        && segment != ".."
}

/// `GET /plugins/{owner}/{name}/{version}/viewer` — build the
/// plugin's viewer extension and stream it as tar.gz (archive root =
/// the version-dir layout: `objectiveai.json`, `viewer/…`). The
/// `X-OBJECTIVEAI-SHA` response header carries the tag's commit SHA.
///
/// Status mapping: 401 unauthenticated; 400 invalid coordinates (the
/// version must be the v-prefixed git tag, `v1.2.3`); 404 tag not
/// found; 503 no laboratory host connected; 500 build failures,
/// message as the plain-text body.
pub(crate) async fn plugin_viewer_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    axum::extract::Path((owner, name, version)): axum::extract::Path<(
        String,
        String,
        String,
    )>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    if !crate::http::daemon_auth::authenticate_header(
        &headers,
        state.global.auth_secret().as_ref(),
    ) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    if !safe_segment(&owner) || !safe_segment(&name) || !safe_segment(&version) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "invalid owner/name/version",
        )
            .into_response();
    }
    let owner = owner.to_lowercase();
    let name = name.to_lowercase();
    // The version IS the git tag, byte-for-byte, Go-modules style —
    // the same rule agent plugin declarations enforce
    // (`agent::plugin::Plugin::validate`). Nothing rewrites it.
    let Some(semver_body) = version.strip_prefix('v') else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "`version` must start with 'v' — it is the plugin repo's git tag, Go-modules style (v1.2.3)",
        )
            .into_response();
    };
    if semver::Version::parse(semver_body).is_err() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            format!("invalid version {version:?}"),
        )
            .into_response();
    }

    let Some(hubs) = state.global.resident_hubs() else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "viewer builds require the resident daemon",
        )
            .into_response();
    };
    let laboratories = hubs.laboratories.clone();
    // The ordinary laboratory load balancer: a uniformly random
    // connected host, independently picked per request.
    let Some((machine, machine_state)) = laboratories.random_host() else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "no laboratory host is connected — run `laboratories spawn` first",
        )
            .into_response();
    };

    // The build runs to completion inside this one forward (which is
    // timeout-free by design — a cold build pulls a base image and
    // installs a dependency tree).
    let built = laboratories
        .forward_to_host(
            &machine,
            &machine_state,
            indexmap::IndexMap::new(),
            RequestPayload::BuildCreate(BuildCreateRequest {
                owner: owner.clone(),
                name: name.clone(),
                version: version.clone(),
            }),
        )
        .await;
    let built: BuildCreated = match built {
        Ok(ResponsePayload::BuildCreate(JsonRpcResult::Ok { result })) => result,
        Ok(ResponsePayload::BuildCreate(JsonRpcResult::Err {
            code, message, ..
        })) => {
            // A missing git tag is the caller's error, and it arrives
            // as its own CODE — no message sniffing.
            let status = if code == BUILD_TAG_NOT_FOUND_CODE {
                axum::http::StatusCode::NOT_FOUND
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            };
            return (status, message).into_response();
        }
        Ok(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "laboratory host answered a viewer build with the wrong payload",
            )
                .into_response();
        }
        Err(e) => {
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
        }
    };

    // Drain the parked archive chunk by chunk straight into the body.
    // A drain error (client disconnect included) just ends the stream:
    // the client sees a truncated gzip and discards. The host retires
    // the artifact on `eof`; an abandoned one is swept host-side.
    let stream = async_stream::stream! {
        let transfer_id = built.transfer_id;
        loop {
            let chunk = laboratories
                .forward_to_host(
                    &machine,
                    &machine_state,
                    indexmap::IndexMap::new(),
                    RequestPayload::BuildRead(TransferIdRequest {
                        transfer_id: transfer_id.clone(),
                    }),
                )
                .await;
            match chunk {
                Ok(ResponsePayload::BuildRead(JsonRpcResult::Ok { result })) => {
                    let eof = result.eof;
                    if !result.data.is_empty() {
                        yield Ok::<_, std::io::Error>(axum::body::Bytes::from(result.data));
                    }
                    if eof {
                        return;
                    }
                }
                Ok(ResponsePayload::BuildRead(JsonRpcResult::Err { message, .. })) => {
                    eprintln!("plugin viewer build: drain: {message}");
                    return;
                }
                Ok(_) => {
                    eprintln!(
                        "plugin viewer build: laboratory host answered a drain with the wrong payload"
                    );
                    return;
                }
                Err(e) => {
                    eprintln!("plugin viewer build: drain: {e}");
                    return;
                }
            }
        }
    };

    (
        [
            (axum::http::header::CONTENT_TYPE, "application/gzip"),
            (
                axum::http::HeaderName::from_static("x-objectiveai-sha"),
                built.commit_sha.as_str(),
            ),
        ],
        axum::body::Body::from_stream(stream),
    )
        .into_response()
}
