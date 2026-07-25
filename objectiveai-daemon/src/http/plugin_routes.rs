//! The daemon's `GET /plugins/{owner}/{name}/{version}/viewer` route:
//! build a plugin's viewer extension on THIS machine and stream the
//! finished artifact back as tar.gz — the build service behind viewer
//! installs (essential for clients that cannot run pnpm/esbuild
//! themselves, e.g. mobile viewers).
//!
//! Flow: validate coordinates → single-tag fetch (local-override
//! first, [`objectiveai_sdk::gitrepo`]) into the daemon's OWN temp
//! partition `<bin>/temp/daemon-viewer` (the laboratory host owns
//! `temp/daemon`, the local viewer `temp/viewer` — each sweeps only
//! its own at boot) → [`crate::viewer_build::build`] into a staging
//! dir shaped as the installed version dir → stream staging as
//! gzip'd tar. The archive is NEVER buffered whole: a blocking task
//! feeds `tar` through a bounded duplex pipe the response body reads
//! from (the `objectiveai-mcp-laboratory` `/export` pattern), so
//! memory stays flat on both ends however large the bundle is.
//! Clients un-tar incrementally into their own staging and land it
//! atomically — a dropped connection mid-stream yields a truncated
//! (invalid) gzip and nothing lands.
//!
//! No lockfile: each request builds in fresh uuid dirs and lands no
//! shared artifact — concurrent identical requests merely duplicate
//! work (a build cache can come later).

use axum::response::IntoResponse;

/// `<bin>/temp/daemon-viewer` — the daemon's build-scratch partition,
/// swept at daemon boot ([`sweep_boot_temp`]).
pub(crate) fn temp_dir(bin_dir: &std::path::Path) -> std::path::PathBuf {
    bin_dir.join("temp").join("daemon-viewer")
}

/// Boot sweep of the daemon's build-scratch partition — hard-killed
/// builds' checkouts and staging dirs. Fresh uuid dirs mean nothing
/// races it.
pub(crate) async fn sweep_boot_temp(bin_dir: &std::path::Path) {
    objectiveai_sdk::gitrepo::sweep_temp(&temp_dir(bin_dir)).await;
}

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
/// found; 500 fetch/build failures, message as the plain-text body.
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

    let bin_dir = state.scoped.filesystem.bin_dir();
    let temp = temp_dir(&bin_dir);
    // `<objectiveai_dir>/plugins` — the local repo override the fetch
    // consults before GitHub.
    let override_dir = state.scoped.filesystem.dir().join("plugins");
    let checkout = match objectiveai_sdk::gitrepo::fetch_at_tag(
        &temp,
        Some(&override_dir),
        &owner,
        &name,
        &version,
    )
    .await
    {
        Ok(checkout) => checkout,
        Err(e) => {
            let status = if e.contains("not found in") {
                axum::http::StatusCode::NOT_FOUND
            } else {
                axum::http::StatusCode::INTERNAL_SERVER_ERROR
            };
            return (status, e).into_response();
        }
    };

    let staging = temp.join(uuid::Uuid::new_v4().to_string());
    let built = crate::viewer_build::build(&checkout.dir, &staging).await;
    // Only staging streams; the checkout is done either way.
    objectiveai_sdk::gitrepo::remove_checkout(&checkout.dir).await;
    if let Err(e) = built {
        objectiveai_sdk::gitrepo::remove_checkout(&staging).await;
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
    }

    // Stream staging as tar.gz through a bounded duplex pipe — sync
    // `tar` on the blocking pool feeding the async body, flat memory
    // (the mcp-laboratory `/export` pattern, plus the gzip layer). A
    // tar/write error (client disconnect included) just drops the
    // writer: the client sees a truncated gzip and discards.
    let (writer, reader) = tokio::io::duplex(64 * 1024);
    let tar_staging = staging.clone();
    let tar_task = tokio::task::spawn_blocking(move || {
        let encoder = flate2::write::GzEncoder::new(
            tokio_util::io::SyncIoBridge::new(writer),
            flate2::Compression::default(),
        );
        let mut builder = tar::Builder::new(encoder);
        let result = builder
            .append_dir_all(".", &tar_staging)
            .and_then(|_| builder.into_inner())
            .and_then(|encoder| encoder.finish().map(|_| ()));
        if let Err(e) = result {
            eprintln!(
                "plugin viewer build: stream tar of {}: {e}",
                tar_staging.display()
            );
        }
    });
    // The staging dir outlives the response body — sweep it once the
    // tar task ends, success and failure alike.
    tokio::spawn(async move {
        let _ = tar_task.await;
        objectiveai_sdk::gitrepo::remove_checkout(&staging).await;
    });

    (
        [
            (axum::http::header::CONTENT_TYPE, "application/gzip"),
            (
                axum::http::HeaderName::from_static("x-objectiveai-sha"),
                checkout.commit_sha.as_str(),
            ),
        ],
        axum::body::Body::from_stream(tokio_util::io::ReaderStream::new(reader)),
    )
        .into_response()
}
