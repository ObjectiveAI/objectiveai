//! Streaming file-transfer endpoints for laboratory-to-laboratory copies.
//!
//! - `GET /export?path=<p>` streams a **tar** archive of `<p>`.
//! - `POST /import?path=<p>` extracts a **tar** archive (the request body) into
//!   `<p>`.
//!
//! Both stream through a sync↔async bridge so nothing buffers the whole archive:
//! tar is a stream format, so the conduit can splice one lab's `/export` straight
//! into another lab's `/import` in a single pass with flat memory. The lab runs
//! inside its container with direct filesystem access, so it tars its own FS —
//! no `podman` involved.
//!
//! No auth in v1: these ride the same loopback-published MCP port the conduit
//! already dials, so they're reachable only by the conduit.
// TODO: peer-to-peer (lab→lab) transfers will need a per-transfer auth token,
// since that exposes these routes beyond the conduit's loopback.

use std::path::PathBuf;

use axum::{
    body::Body,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use serde::Deserialize;
use tokio_util::io::{ReaderStream, StreamReader, SyncIoBridge};

#[derive(Deserialize)]
pub struct PathQuery {
    /// Absolute path inside the container to export from / import into.
    path: String,
}

/// `GET /export?path=<p>` — stream a tar of the file or directory at `path`.
/// The archive records the entry under its basename, so an importer recreates
/// `<dest>/<basename>` (cp-style semantics).
pub async fn export(Query(q): Query<PathQuery>) -> Response {
    let path = PathBuf::from(&q.path);
    let name = match path.file_name() {
        Some(n) => n.to_owned(),
        None => return (StatusCode::BAD_REQUEST, "path has no file name").into_response(),
    };
    let metadata = match tokio::fs::metadata(&path).await {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::NOT_FOUND, format!("stat {}: {e}", path.display()))
                .into_response();
        }
    };
    let is_dir = metadata.is_dir();

    // Bridge sync `tar` → async body via an in-memory duplex pipe: the builder
    // runs on a blocking thread writing into the pipe; the read half streams out
    // as the HTTP body. The bounded pipe gives backpressure → flat memory. The
    // bridge must be created in async context (it captures the runtime handle).
    let (writer, reader) = tokio::io::duplex(64 * 1024);
    let bridge = SyncIoBridge::new(writer);
    tokio::task::spawn_blocking(move || {
        let mut builder = tar::Builder::new(bridge);
        let result = if is_dir {
            builder.append_dir_all(&name, &path)
        } else {
            builder.append_path_with_name(&path, &name)
        };
        // On error the stream is dropped truncated → the importer's `unpack`
        // fails, surfacing the failure end-to-end. Log for diagnosis.
        if let Err(e) = result.and_then(|_| builder.finish().map(|_| ())) {
            tracing::warn!("laboratory export tar of {}: {e}", path.display());
        }
    });

    Body::from_stream(ReaderStream::new(reader)).into_response()
}

/// `POST /import?path=<p>` — extract the request body's tar into `path`,
/// creating it if needed. Streams in as bytes arrive.
pub async fn import(Query(q): Query<PathQuery>, body: Body) -> Response {
    let dest = PathBuf::from(&q.path);
    if let Err(e) = tokio::fs::create_dir_all(&dest).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("create_dir_all {}: {e}", dest.display()),
        )
            .into_response();
    }

    // Body stream → AsyncRead → sync bridge → `tar` unpack on a blocking thread.
    let stream = body.into_data_stream().map_err(std::io::Error::other);
    let bridge = SyncIoBridge::new(StreamReader::new(stream));
    let result = tokio::task::spawn_blocking(move || {
        tar::Archive::new(bridge).unpack(&dest)
    })
    .await;

    match result {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, format!("unpack: {e}")).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")).into_response(),
    }
}
