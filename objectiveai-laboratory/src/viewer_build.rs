//! Viewer-plugin builds: the plugin's OWN build, and the parked
//! artifacts it produces.
//!
//! One build = fetch the plugin repo at its version's git tag →
//! `podman build` the manifest's `viewer_containerfile` with that
//! checkout as context (verbatim the flow
//! [`crate::plugin_image::ensure`] runs for the plugin's MCP image) →
//! copy `viewer_output`'s contents out of the resulting image → pack
//! them with the manifest → hand the daemon a drain handle.
//!
//! The image is never RUN. A viewer build's work happens in `RUN`
//! steps at image-build time, so all that remains is to open a
//! filesystem view of the result: create a container, `podman cp`, and
//! remove the container AND the image. Nothing this build made
//! survives it but the parked artifact — the plugin's own base image
//! stays cached and shared, which is the only part worth keeping.
//!
//! Because the plugin owns its toolchain, the ONE invariant we cannot
//! enforce is the author's: react and its subpath specifiers must be
//! left external (the host viewer serves them through an import map).
//! What we CAN check, and do, is that the build actually produced
//! every file the manifest promises — see [`validate_output`].
//!
//! Why the artifact is parked rather than returned inline: the archive
//! is unbounded (a plugin bundling a big library is megabytes), and
//! the wire's chunk lane already exists for exactly this.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use objectiveai_sdk::cli::plugins::{Manifest, ViewerTab};

use crate::plugin_image::PluginCoords;
use crate::podman::{self, Podman};

/// Raw bytes per drain chunk — the transfer registry's size, for the
/// same reason (chunks ride binary frames, never base64).
const CHUNK_SIZE: usize = 2 * 1024 * 1024;

/// A parked artifact untouched this long was abandoned by its
/// draining daemon — swept lazily on every park.
const IDLE_SECS: i64 = 300;

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// One finished build's archive, waiting to be drained.
struct Artifact {
    /// The scratch dir holding the archive — removed with the entry.
    dir: PathBuf,
    file: tokio::sync::Mutex<Option<tokio::fs::File>>,
    last_used: std::sync::atomic::AtomicI64,
}

/// The host's parked build artifacts, keyed by minted transfer id.
#[derive(Default)]
pub struct BuildArtifacts {
    entries: DashMap<String, Arc<Artifact>>,
}

impl BuildArtifacts {
    /// Park a finished archive and mint its drain handle. Sweeps
    /// abandoned entries first — a daemon that died mid-drain leaves
    /// one behind, and nothing else would ever collect it.
    async fn park(&self, dir: PathBuf, archive: &Path) -> Result<String, String> {
        let stale: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| {
                now_secs()
                    - entry
                        .value()
                        .last_used
                        .load(std::sync::atomic::Ordering::Relaxed)
                    >= IDLE_SECS
            })
            .map(|entry| entry.key().clone())
            .collect();
        for id in stale {
            self.discard(&id).await;
        }
        let file = tokio::fs::File::open(archive)
            .await
            .map_err(|e| format!("open build artifact: {e}"))?;
        let transfer_id = uuid::Uuid::new_v4().to_string();
        self.entries.insert(
            transfer_id.clone(),
            Arc::new(Artifact {
                dir,
                file: tokio::sync::Mutex::new(Some(file)),
                last_used: std::sync::atomic::AtomicI64::new(now_secs()),
            }),
        );
        Ok(transfer_id)
    }

    /// Drain the next chunk. `eof` retires the entry and its scratch
    /// dir, so a complete drain leaves nothing behind.
    pub async fn read(&self, transfer_id: &str) -> Result<(Vec<u8>, bool), String> {
        use tokio::io::AsyncReadExt as _;
        let entry = match self.entries.get(transfer_id) {
            Some(entry) => Arc::clone(&entry),
            None => return Err(format!("no build artifact '{transfer_id}'")),
        };
        entry
            .last_used
            .store(now_secs(), std::sync::atomic::Ordering::Relaxed);
        let mut guard = entry.file.lock().await;
        let Some(file) = guard.as_mut() else {
            return Err(format!("build artifact '{transfer_id}' already drained"));
        };
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut filled = 0usize;
        while filled < CHUNK_SIZE {
            match file.read(&mut buf[filled..]).await {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) => {
                    *guard = None;
                    drop(guard);
                    self.discard(transfer_id).await;
                    return Err(format!("read build artifact: {e}"));
                }
            }
        }
        buf.truncate(filled);
        let eof = filled < CHUNK_SIZE;
        if eof {
            *guard = None;
            drop(guard);
            self.discard(transfer_id).await;
        }
        Ok((buf, eof))
    }

    /// Drop a parked artifact and its scratch dir. Idempotent.
    pub async fn discard(&self, transfer_id: &str) {
        if let Some((_, artifact)) = self.entries.remove(transfer_id) {
            let _ = tokio::fs::remove_dir_all(&artifact.dir).await;
        }
    }
}

/// Why a build didn't produce an artifact.
pub enum BuildFailure {
    /// The plugin's git tag does not exist — the caller's error, the
    /// daemon's `404`.
    TagNotFound(String),
    /// Anything else: a malformed manifest, the image build failing,
    /// or output that doesn't match what the manifest promised.
    Failed(String),
}

/// A finished build, ready to drain.
pub struct Built {
    pub commit_sha: String,
    pub transfer_id: String,
    pub bytes: u64,
}

/// The viewer half of a manifest, validated: all three fields present
/// together, or the plugin has no viewer extension at all.
struct ViewerSpec {
    /// Where the built assets land in the installed layout.
    root: String,
    /// The resolved Containerfile that builds them.
    containerfile: PathBuf,
    /// The path inside the built image to copy out.
    output: String,
}

/// Validate a repo-relative forward-slash path and resolve it under
/// `root` — the containerfile rules, mirroring
/// [`crate::plugin_manifest`]'s.
fn resolve_repo_rel(root: &Path, path: &str, what: &str) -> Result<PathBuf, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err(format!("`{what}` cannot be empty"));
    }
    if path.contains('\\') {
        return Err(format!("`{what}` must use forward slashes"));
    }
    if path.starts_with('/') || path.contains(':') {
        return Err(format!("`{what}` must be a repo-relative path"));
    }
    if path
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(format!("`{what}` has an invalid path component: {path:?}"));
    }
    Ok(path
        .split('/')
        .fold(root.to_path_buf(), |path, component| path.join(component)))
}

/// A layout-relative path (a tab `module` or the `icon`), authored
/// CWD-style against the viewer root. `None` = it tries to leave.
fn normalize_layout_path(path: &str) -> Option<String> {
    let path = path.strip_prefix("./").unwrap_or(path);
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains("://")
        || path
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
    {
        return None;
    }
    Some(path.to_string())
}

/// Existence probe. Sync: this runs a handful of times per build,
/// around IO that already blocked.
fn is_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file())
}

/// The viewer half of the checkout's manifest. `Ok(None)` = the plugin
/// declares no viewer extension; `Err` = it declares half of one.
fn viewer_spec(
    checkout: &Path,
    manifest: &Manifest,
) -> Result<Option<ViewerSpec>, String> {
    let Some(root) = manifest.viewer.as_deref() else {
        return Ok(None);
    };
    let root = root.trim();
    if normalize_layout_path(root).is_none() {
        return Err(format!("`viewer` is not a valid layout directory: {root:?}"));
    }
    let Some(containerfile) = manifest.viewer_containerfile.as_deref() else {
        return Err(
            "plugin declares `viewer` but no `viewer_containerfile` — the Containerfile that builds the viewer extension"
                .to_string(),
        );
    };
    let Some(output) = manifest.viewer_output.as_deref() else {
        return Err(
            "plugin declares `viewer` but no `viewer_output` — the path inside the built image whose contents are the built assets"
                .to_string(),
        );
    };
    let output = output.trim();
    if !output.starts_with('/') || output.contains("..") {
        return Err(format!(
            "`viewer_output` must be an absolute in-container path: {output:?}"
        ));
    }
    let containerfile =
        resolve_repo_rel(checkout, containerfile, "viewer_containerfile")?;
    if !is_file(&containerfile) {
        return Err(format!(
            "`viewer_containerfile` not found in the repo: {}",
            containerfile.display()
        ));
    }
    Ok(Some(ViewerSpec {
        root: root.to_string(),
        containerfile,
        output: output.to_string(),
    }))
}

/// Every file the manifest promises must actually be in the copied
/// output. The author owns the build, so this is where a build that
/// "succeeded" while producing nothing usable is caught — here, not in
/// the user's viewer.
fn validate_output(
    staging: &Path,
    spec: &ViewerSpec,
    manifest: &Manifest,
) -> Result<(), String> {
    let root = staging.join(&spec.root);
    let check = |relative: &str, what: &str| -> Result<(), String> {
        let Some(normalized) = normalize_layout_path(relative) else {
            return Err(format!("invalid {what} path {relative:?}"));
        };
        let target = normalized
            .split('/')
            .fold(root.clone(), |path, component| path.join(component));
        if is_file(&target) {
            Ok(())
        } else {
            Err(format!(
                "{what} {relative:?} is not in the build's output ({}/{normalized})",
                spec.root
            ))
        }
    };
    for tab in manifest.tabs.iter().flatten() {
        let module = match tab {
            ViewerTab::Channel { module, .. } | ViewerTab::Tab { module, .. } => module,
        };
        check(module, "tab module")?;
    }
    if let Some(icon) = manifest.icon.as_deref() {
        check(icon, "icon")?;
    }
    Ok(())
}

/// Pack the staging dir as the archive the daemon streams verbatim:
/// its root IS the installed version dir. Returns the archive's size.
async fn pack(staging: &Path, archive: &Path) -> Result<u64, String> {
    let staging = staging.to_path_buf();
    let archive = archive.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&archive)
            .map_err(|e| format!("create build archive: {e}"))?;
        let encoder =
            flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        builder
            .append_dir_all(".", &staging)
            .and_then(|_| builder.into_inner())
            .and_then(|encoder| encoder.finish())
            .map_err(|e| format!("pack build archive: {e}"))?;
        std::fs::metadata(&archive)
            .map(|m| m.len())
            .map_err(|e| format!("stat build archive: {e}"))
    })
    .await
    .map_err(|e| format!("pack build archive: {e}"))?
}

/// Run one viewer-extension build to completion.
///
/// Everything transient is cleaned on every path: the checkout, the
/// export container, and the built image. Only the artifact's scratch
/// dir survives, owned by the returned handle until it is drained or
/// swept.
pub async fn build(
    podman: &Podman,
    bin_dir: &Path,
    artifacts: &BuildArtifacts,
    owner: &str,
    name: &str,
    version: &str,
) -> Result<Built, BuildFailure> {
    let coords =
        PluginCoords::canonicalize(owner, name, version).map_err(BuildFailure::Failed)?;
    let temp = bin_dir.join("temp").join("build");
    // `<objectiveai_dir>/plugins` — bin_dir is always
    // `<objectiveai_dir>/bin`, and the local override lets an author
    // build an unpushed tag.
    let checkout = objectiveai_sdk::gitrepo::fetch_at_tag(
        &temp,
        bin_dir.parent().map(|dir| dir.join("plugins")).as_deref(),
        &coords.owner,
        &coords.name,
        coords.git_tag(),
    )
    .await
    .map_err(|e| {
        // `gitrepo` reports a missing tag as `"plugin tag '…' not
        // found in <owner>/<name>"`. Classifying it HERE — at the
        // boundary that owns the fetch — is what lets the daemon key
        // its 404 off an error CODE instead of re-sniffing prose.
        if e.contains("not found in") {
            BuildFailure::TagNotFound(e)
        } else {
            BuildFailure::Failed(e)
        }
    })?;

    // One work dir per build, holding the staged LAYOUT and, beside
    // it, the archive packed from that layout — beside, never inside,
    // or the archive would tar itself.
    let work = temp.join(uuid::Uuid::new_v4().to_string());
    let packed = run(podman, &checkout, &work).await;
    // The checkout is transient scratch — gone the moment the build
    // concludes, success or failure.
    objectiveai_sdk::gitrepo::remove_checkout(&checkout.dir).await;
    let landed = match packed {
        Ok((archive, bytes)) => artifacts
            .park(work.clone(), &archive)
            .await
            .map(|transfer_id| Built {
                commit_sha: checkout.commit_sha.clone(),
                transfer_id,
                bytes,
            })
            .map_err(BuildFailure::Failed),
        Err(e) => Err(e),
    };
    // The work dir belongs to the parked artifact from here — but only
    // if one was actually parked.
    if landed.is_err() {
        let _ = tokio::fs::remove_dir_all(&work).await;
    }
    landed
}

/// The fallible middle of [`build`] — everything between a fetched
/// checkout and a packed archive. Returns `(archive path, bytes)`.
async fn run(
    podman: &Podman,
    checkout: &objectiveai_sdk::gitrepo::CheckedOutRepo,
    work: &Path,
) -> Result<(PathBuf, u64), BuildFailure> {
    // The installed version dir, assembled: the archive's root.
    let staging = work.join("layout");
    let staging = staging.as_path();
    let (manifest, _) = crate::plugin_manifest::read(&checkout.dir)
        .await
        .map_err(BuildFailure::Failed)?;
    let Some(spec) =
        viewer_spec(&checkout.dir, &manifest).map_err(BuildFailure::Failed)?
    else {
        return Err(BuildFailure::Failed(
            "plugin declares no viewer extension (`viewer` absent from objectiveai.json)"
                .to_string(),
        ));
    };

    // An EPHEMERAL tag: the image exists only to be copied out of, and
    // two builds of one plugin version may legitimately run at once.
    let nonce = uuid::Uuid::new_v4().to_string();
    let image = format!("localhost/objectiveai-viewer-build:{nonce}");
    let container = format!("objectiveai-viewer-build-{nonce}");
    podman::laboratory::image_build(
        podman,
        &spec.containerfile,
        &checkout.dir,
        &image,
        &[],
    )
    .await
    .map_err(|e| BuildFailure::Failed(e.0))?;

    // From here the IMAGE exists — it must be removed on every path.
    let extracted = async {
        let root = staging.join(&spec.root);
        tokio::fs::create_dir_all(&root)
            .await
            .map_err(|e| BuildFailure::Failed(format!("build staging dir: {e}")))?;
        podman::laboratory::create_for_export(podman, &container, &image)
            .await
            .map_err(|e| BuildFailure::Failed(e.0))?;
        // From here the CONTAINER exists too.
        let copied =
            podman::laboratory::copy_out(podman, &container, &spec.output, &root)
                .await
                .map_err(|e| BuildFailure::Failed(e.0));
        if let Err(e) = podman::laboratory::remove_named(podman, &container).await {
            eprintln!("viewer build: remove {container}: {}", e.0);
        }
        copied?;
        // The manifest we validated IS the manifest that installs —
        // staged from the checkout, so an author can neither forget to
        // ship it nor ship a different one.
        tokio::fs::copy(
            checkout.dir.join(crate::plugin_manifest::MANIFEST_FILE),
            staging.join(crate::plugin_manifest::MANIFEST_FILE),
        )
        .await
        .map_err(|e| BuildFailure::Failed(format!("stage manifest: {e}")))?;
        validate_output(staging, &spec, &manifest).map_err(BuildFailure::Failed)?;
        let archive = work.join("bundle.tar.gz");
        let bytes = pack(staging, &archive).await.map_err(BuildFailure::Failed)?;
        Ok((archive, bytes))
    }
    .await;
    // The built image was only ever a carrier — removing it frees the
    // layers THIS build added, while the plugin's own base image stays
    // tagged and shared for the next one.
    if let Err(e) = podman::laboratory::image_remove(podman, &image).await {
        eprintln!("viewer build: remove image {image}: {}", e.0);
    }
    extracted
}
