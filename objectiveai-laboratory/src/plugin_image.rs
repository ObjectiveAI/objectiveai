//! Plugin image identity + the build-or-reuse ensure flow.
//!
//! ONE plugin = ONE MCP server = ONE image per canonical
//! `(owner, name, version)`. The image tag pins the exact bits: if it
//! exists locally, it is used as-is (its labels carry the port/SHA
//! metadata, so the fast path never re-clones); if not, the repo is
//! fetched at the version's git tag, built, tagged, and the checkout
//! deleted — all under a machine-wide bin lock so no two laboratory
//! hosts clone/build the same plugin at once.

use std::path::Path;

use crate::podman::{self, Podman};

/// Image label carrying the plugin's container-internal MCP port.
const PORT_LABEL: &str = "objectiveai.plugin.port";
/// Image label carrying the manifest's `mcp.postgres` opt-in
/// (`"true"`/`"false"`). The whole db-proxy chain — injection, port
/// publish, host dial, `OBJECTIVEAI_POSTGRES_URL` — hangs off it, and
/// it rides the image exactly as the port does so the fast path never
/// re-reads a manifest.
const POSTGRES_LABEL: &str = "objectiveai.plugin.postgres";
/// Image label carrying the git commit SHA the image built from.
const SHA_LABEL: &str = "objectiveai.plugin.sha";
/// Image label carrying the DEVELOPMENT source directory an image was
/// built from — present IFF the bits came off a registered local
/// directory rather than a git tag, and absent for every released
/// plugin. Its VALUE is that directory, so the fast path can tell
/// "built from THIS registration" from "built from a DIFFERENT one"
/// and from "built from git" — all three share one image tag. See
/// [`reusable`].
const DEVELOPMENT_LABEL: &str = "objectiveai.plugin.development";

/// Where development build caches live: a sibling of `locks/`,
/// `plugins/` and `temp/` under `<objectiveai_dir>/bin`.
///
/// Deliberately NOT under `<bin>/temp`, which `cleaner::sweep` walks
/// at every host boot and empties — a cache parked there would be
/// garbage-collected by a sibling subsystem for reasons neither could
/// see.
const CACHE_DIR: &str = "plugin-cache";

/// A plugin's CANONICAL coordinates: `owner`/`name` lowercased (GitHub
/// treats them case-insensitively), `version` VERBATIM — it IS the
/// repo's git tag, Go-modules style (`v1.2.3`, case-sensitive). The
/// `v` prefix is REQUIRED at the declaration layer
/// (`agent::plugin::validate`) and never rewritten here — no prepend,
/// no check.
///
/// Everything derives from this trio:
/// - laboratory id: `oai-plugin-{owner}-{name}-{version}` — NOTE the
///   `-` joins are ambiguous across the trio (owner `a-b`/name `c` vs
///   owner `a`/name `b-c`); accepted-degenerate, the container label
///   stores the exact trio.
/// - image: `localhost/objectiveai-plugin:{owner}-{name}-{version}` —
///   the coordinates ride the TAG part, whose charset (unlike the
///   lowercase-only name path) tolerates repo names like `.github`.
/// - git tag: `{version}`, byte-for-byte.
pub struct PluginCoords {
    pub owner: String,
    pub name: String,
    pub version: String,
}

impl PluginCoords {
    /// Canonicalize + validate a declared trio. Segment charset is the
    /// intersection of what container names and image tags accept —
    /// `[a-z0-9._-]` after lowercasing owner/name (version also allows
    /// uppercase, which the tag part and container names both accept);
    /// this rejects `/`, `:`, `+` (semver build metadata), whitespace,
    /// and non-ASCII outright.
    pub fn canonicalize(owner: &str, name: &str, version: &str) -> Result<Self, String> {
        let owner = owner.trim().to_lowercase();
        let name = name.trim().to_lowercase();
        let version = version.trim().to_string();
        for (label, value, allow_upper) in [
            ("owner", &owner, false),
            ("name", &name, false),
            ("version", &version, true),
        ] {
            if value.is_empty() {
                return Err(format!("plugin `{label}` cannot be empty"));
            }
            if !value.chars().all(|c| {
                c.is_ascii_lowercase()
                    || (allow_upper && c.is_ascii_uppercase())
                    || c.is_ascii_digit()
                    || matches!(c, '.' | '_' | '-')
            }) {
                return Err(format!(
                    "plugin `{label}` {value:?} has characters outside [a-z0-9._-]",
                ));
            }
        }
        let coords = Self { owner, name, version };
        // The whole coordinate part must fit the image-tag grammar's
        // 128-char cap (and the id must stay one sane path segment).
        if coords.image_tag().len() > 128 {
            return Err(format!(
                "plugin coordinates too long for an image tag: {:?}",
                coords.image_tag(),
            ));
        }
        Ok(coords)
    }

    /// The derived laboratory id (the coordinate part, without a
    /// response id).
    pub fn laboratory_id(&self) -> String {
        format!(
            "{}{}-{}-{}",
            objectiveai_sdk::laboratories::PLUGIN_LABORATORY_ID_PREFIX,
            self.owner,
            self.name,
            self.version,
        )
    }

    /// The EPHEMERAL laboratory id for ONE agent-completion response:
    /// the coordinate id plus the response id. Ephemeral laboratories
    /// live exactly as long as their single MCP connection.
    pub fn ephemeral_laboratory_id(&self, response_id: &str) -> String {
        format!("{}-{response_id}", self.laboratory_id())
    }

    /// The image reference's TAG part.
    pub fn image_tag(&self) -> String {
        format!("{}-{}-{}", self.owner, self.name, self.version)
    }

    /// The split image reference (`localhost/objectiveai-plugin` +
    /// tag) — what the container label records, list display included.
    pub fn image(&self) -> objectiveai_sdk::laboratories::RegistryLaboratoryImage {
        objectiveai_sdk::laboratories::RegistryLaboratoryImage {
            registry: "localhost".to_string(),
            name: "objectiveai-plugin".to_string(),
            pin: objectiveai_sdk::laboratories::LaboratoryImagePin::Tag(
                self.image_tag(),
            ),
        }
    }

    /// The joined reference (`localhost/objectiveai-plugin:{tag}`) for
    /// podman image commands.
    pub fn image_reference(&self) -> String {
        format!("localhost/objectiveai-plugin:{}", self.image_tag())
    }

    /// The git tag the version names — the version string itself,
    /// byte-for-byte (Go modules: tag == version).
    pub fn git_tag(&self) -> &str {
        &self.version
    }
}

/// What [`ensure`] recovers about the plugin's image, whichever path
/// ran: the container-internal MCP port (from the manifest at build
/// time, from the image labels on the fast path) and the commit SHA.
pub struct EnsuredPluginImage {
    pub port: u16,
    /// The manifest's `mcp.postgres` opt-in, off [`POSTGRES_LABEL`] on
    /// the fast path. Decides whether the container gets the db proxy
    /// at all.
    pub postgres: bool,
    pub sha: Option<String>,
}

/// Why a development build could not start: the developer's own
/// registration is wrong, not the plugin and not this host.
///
/// Carried as a distinct type so `host.rs` can answer with
/// [`PLUGIN_DEVELOPMENT_SOURCE_CODE`][objectiveai_sdk::laboratories::daemon::PLUGIN_DEVELOPMENT_SOURCE_CODE]
/// instead of a generic internal error — the caller should be told to
/// fix or drop the registration, not handed a 500.
pub struct DevelopmentSourceError(pub String);

/// Ensure the plugin's image exists locally, building it if needed.
///
/// - Fast path: `podman image exists` AND the image was built from the
///   source THIS call wants (see [`reusable`]) → read the metadata off
///   its labels, no fetch.
/// - Build path: take the machine-wide bin lock, RE-CHECK (a sibling
///   host may have finished the build while we waited), then build —
///   and release the lock on EVERY path, since a `LockClaim` drop
///   deliberately does NOT release (`podman/install.rs` pattern).
///
/// `development` is the registered host directory that stands in for
/// the git checkout. `Some` ⇒ nothing is fetched and NOTHING IS
/// DELETED — the tree belongs to the developer, and that is the only
/// reason the production arm's `remove_checkout` was unconditional —
/// and the manifest's `mcp.development.caches` are bound in as build
/// volumes.
pub async fn ensure(
    podman: &Podman,
    bin_dir: &Path,
    coords: &PluginCoords,
    development: Option<&Path>,
) -> Result<EnsuredPluginImage, String> {
    let reference = coords.image_reference();
    if let Some(ensured) = reusable(podman, &reference, development).await? {
        return Ok(ensured);
    }
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        &format!("plugin-image-{}", coords.image_tag()),
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| format!("bin lock: {e}"))?;
    let result = async {
        // Double-checked: a sibling may have built + tagged while we
        // were blocked on the lock.
        if let Some(ensured) = reusable(podman, &reference, development).await? {
            return Ok(ensured);
        }
        match development {
            // DEVELOPMENT: the registered directory IS the source tree.
            Some(dir) => {
                check_development_dir(dir).await.map_err(|e| e.0)?;
                build(podman, bin_dir, coords, dir, None, Some(dir)).await
            }
            // PRODUCTION: fetch, build, delete — the fetch and the
            // delete both live in THIS arm, which is the whole reason
            // the two modes can share one function.
            None => {
                // `<objectiveai_dir>/plugins` — bin_dir is always
                // `<objectiveai_dir>/bin` (main.rs derives it that
                // way). The daemon-side temp subtree is
                // `<bin>/temp/daemon`; the viewer installer owns
                // `<bin>/temp/viewer`.
                let checkout = objectiveai_sdk::gitrepo::fetch_at_tag(
                    &bin_dir.join("temp").join("daemon"),
                    bin_dir.parent().map(|dir| dir.join("plugins")).as_deref(),
                    &coords.owner,
                    &coords.name,
                    coords.git_tag(),
                )
                .await?;
                let built = build(
                    podman,
                    bin_dir,
                    coords,
                    &checkout.dir,
                    Some(&checkout.commit_sha),
                    None,
                )
                .await;
                // The checkout is transient scratch — gone the moment
                // the build concludes, success or failure.
                objectiveai_sdk::gitrepo::remove_checkout(&checkout.dir).await;
                built
            }
        }
    }
    .await;
    claim
        .release()
        .map_err(|e| format!("bin lock release: {e}"))?;
    result
}

/// Read the manifest at `root`, resolve the containerfile, and build.
///
/// `root` is a git checkout (production) or the registered directory
/// (development); `development` is `Some(root)` in the latter, which
/// is what turns the caches on and stamps [`DEVELOPMENT_LABEL`].
/// Always runs under the image lock.
async fn build(
    podman: &Podman,
    bin_dir: &Path,
    coords: &PluginCoords,
    root: &Path,
    commit_sha: Option<&str>,
    development: Option<&Path>,
) -> Result<EnsuredPluginImage, String> {
    let manifest = crate::plugin_manifest::read(root).await?;
    // A plugin may legitimately ship only a viewer extension. Declaring
    // one as an agent's MCP plugin is the caller's mistake, and THIS is
    // the first point that can see it — the API, proxy and daemon only
    // ever handle coordinates. Nothing caches the refusal: there is no
    // image to find on the fast path, so the next request re-reads and
    // fails identically.
    let Some(mcp) = manifest.mcp.as_ref() else {
        return Err(
            "plugin declares no MCP server (`mcp` absent from objectiveai.json)".to_string(),
        );
    };
    if mcp.port == 0 {
        return Err("plugin manifest: `mcp.port` cannot be 0".to_string());
    }
    let file = crate::plugin_manifest::resolve_build_file(
        root,
        &mcp.containerfile,
        "mcp.containerfile",
    )
    .await?;

    let mut labels = vec![
        (PORT_LABEL.to_string(), mcp.port.to_string()),
        (POSTGRES_LABEL.to_string(), mcp.postgres.to_string()),
    ];
    let mut volumes = Vec::new();
    match development {
        Some(dir) => {
            // No SHA label: there is no commit. `EnsuredPluginImage.sha`
            // and the container's `PluginLabel.sha` are both already
            // `Option`, so absence flows end to end — where a
            // fabricated "development" would eventually be read,
            // logged, or displayed AS a commit.
            labels.push((
                DEVELOPMENT_LABEL.to_string(),
                dir.to_string_lossy().into_owned(),
            ));
            volumes = cache_mounts(bin_dir, coords, mcp.development.as_ref()).await?;
        }
        None => {
            if let Some(sha) = commit_sha {
                labels.push((SHA_LABEL.to_string(), sha.to_string()));
            }
            // `mcp.development` is INERT here: a released plugin's
            // build never binds a host directory.
        }
    }

    podman::laboratory::image_build(
        podman,
        &file.containerfile,
        &file.context,
        &coords.image_reference(),
        &labels,
        &volumes,
    )
    .await
    .map_err(|e| e.0)?;

    Ok(EnsuredPluginImage {
        port: mcp.port,
        postgres: mcp.postgres,
        sha: commit_sha.map(str::to_string),
    })
}

/// The exists-fast-path read, MODE-CHECKED: `Some` only when the
/// tagged image exists AND was built from the source this call
/// actually wants.
///
/// The check is what makes one image tag safe to share between a git
/// tag and a local directory — the tag is derived from coordinates,
/// and development changes only where the bits come from. Without it,
/// registering a directory, building, then unregistering would leave
/// locally-built bits under the tag that the PRODUCTION path
/// fast-paths into forever, shipping uncommitted code under a
/// git-tag identity. A mismatch reads as ABSENT, so the build path
/// retags and self-heals with no cooperation from the daemon and no
/// reset that might never arrive.
///
/// The port label stays REQUIRED: an image under our tag without it
/// was not built by this flow, and guessing a port is worse than
/// refusing.
async fn reusable(
    podman: &Podman,
    reference: &str,
    development: Option<&Path>,
) -> Result<Option<EnsuredPluginImage>, String> {
    if !podman::laboratory::image_exists(podman, reference)
        .await
        .map_err(|e| e.0)?
    {
        return Ok(None);
    }
    let labels = podman::laboratory::image_labels(podman, reference)
        .await
        .map_err(|e| e.0)?;
    // Byte-exact. A re-registration differing only in spelling (case,
    // a trailing separator) costs ONE extra rebuild — cheaper than
    // teaching this per-platform path equality, and it errs toward
    // rebuilding rather than toward reusing the wrong bits.
    let stamped = labels.get(DEVELOPMENT_LABEL).map(String::as_str);
    let wanted = development.map(|dir| dir.to_string_lossy());
    if stamped != wanted.as_deref() {
        return Ok(None);
    }
    let port = labels
        .get(PORT_LABEL)
        .ok_or_else(|| format!("image {reference} is missing the {PORT_LABEL} label"))?;
    let port: u16 = port
        .parse()
        .map_err(|e| format!("image {reference} {PORT_LABEL} label: {e}"))?;
    if port == 0 {
        return Err(format!("image {reference} {PORT_LABEL} label is 0"));
    }
    // Required for the same reason as the port, with the same remedy
    // (rebuild — `development plugins mcp reset` for a dev image): an
    // image without it predates the opt-in or was not built by this
    // flow, and guessing whether a plugin gets a database is worse
    // than refusing.
    let postgres = labels
        .get(POSTGRES_LABEL)
        .ok_or_else(|| format!("image {reference} is missing the {POSTGRES_LABEL} label"))?;
    let postgres: bool = postgres
        .parse()
        .map_err(|e| format!("image {reference} {POSTGRES_LABEL} label: {e}"))?;
    Ok(Some(EnsuredPluginImage {
        port,
        postgres,
        sha: labels.get(SHA_LABEL).cloned(),
    }))
}

/// The host directory holding every cache for one plugin.
///
/// `image_tag()` is ONE path segment on purpose. `PluginCoords::
/// canonicalize` permits `.` freely, so `version = ".."` passes
/// validation today — harmless while nothing uses the trio as path
/// segments, but an `owner/name/version` layout would make it a live
/// traversal out of `<bin>`. `{owner}-{name}-{version}` always
/// contains at least two `-`, so it can never BE `.` or `..`, never
/// holds a separator, is `[a-z0-9._-]` only (excluding every
/// Windows-reserved character), and is capped at 128 chars by
/// `canonicalize`. It is also already the lock key and the image tag —
/// one namespace, three uses.
fn plugin_cache_dir(bin_dir: &Path, coords: &PluginCoords) -> std::path::PathBuf {
    bin_dir.join(CACHE_DIR).join(coords.image_tag())
}

/// The host directory backing ONE declared container cache path.
///
/// `{label}-{hash}`: the hash is the first 16 hex of SHA-256 over the
/// container path's exact bytes and is what makes this deterministic
/// and collision-free; the label is decoration so `ls` is readable.
/// The label maps everything outside `[a-z0-9]` to `-` and truncates,
/// which also strips every dot — so the name can never be `.`/`..`,
/// never leads or trails with a dot (Windows silently drops those),
/// and being `-`-joined to hex can never collide with a reserved
/// device name like `con` or `nul`.
fn cache_slug(container_path: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut label = String::new();
    let mut last_dash = true;
    for ch in container_path.chars() {
        if ch.is_ascii_alphanumeric() {
            label.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            label.push('-');
            last_dash = true;
        }
        if label.len() >= 24 {
            break;
        }
    }
    let label = label.trim_matches('-');
    let hash = hex::encode(Sha256::digest(container_path.as_bytes()));
    format!("{label}-{}", &hash[..16])
}

/// Validate the manifest's declared caches and materialize their host
/// directories, returning the build-time bind mounts.
///
/// Runs under the image lock, so two concurrent creates never race the
/// `create_dir_all` and — the part that actually matters — never run
/// two builds against one cache. Created explicitly rather than left
/// to podman: podman creating a bind source can leave it owned by a
/// uid the host user cannot later delete, and a real failure (a file
/// squatting the path, a read-only disk) deserves a named error rather
/// than a confusing build failure.
async fn cache_mounts(
    bin_dir: &Path,
    coords: &PluginCoords,
    development: Option<&objectiveai_sdk::cli::plugins::McpDevelopment>,
) -> Result<Vec<podman::laboratory::Mount>, String> {
    let Some(development) = development else {
        return Ok(Vec::new());
    };
    // Lexical validation lives in the SDK so the CLI's manifest lint
    // catches it too; this is the second line, not the first.
    development.validate()?;
    let root = plugin_cache_dir(bin_dir, coords);
    let mut mounts = Vec::with_capacity(development.caches.len());
    for container in &development.caches {
        let host = root.join(cache_slug(container));
        tokio::fs::create_dir_all(&host)
            .await
            .map_err(|e| format!("create plugin cache {}: {e}", host.display()))?;
        mounts.push(podman::laboratory::Mount {
            host: host.to_string_lossy().into_owned(),
            container: container.clone(),
        });
    }
    Ok(mounts)
}

/// Drop the plugin's tagged image so the next create rebuilds, and
/// optionally its development caches.
///
/// Under the SAME `plugin-image-{tag}` lock [`ensure`] builds under, so
/// a reset can never land mid-build nor race the double-checked fast
/// path.
///
/// Removing nothing is a SUCCESS: the command's purpose is that the
/// next run rebuilds, and for a plugin that was never built that is
/// already true.
pub async fn reset(
    podman: &Podman,
    bin_dir: &Path,
    coords: &PluginCoords,
    caches: bool,
) -> Result<objectiveai_sdk::laboratories::daemon::PluginImageResetResult, String> {
    let reference = coords.image_reference();
    let claim = objectiveai_sdk::lockfile::wait_acquire(
        &bin_dir.join("locks"),
        &format!("plugin-image-{}", coords.image_tag()),
        &format!("pid {}", std::process::id()),
    )
    .await
    .map_err(|e| format!("bin lock: {e}"))?;
    let result = async {
        // Under the lock, so this cannot go stale before the removal.
        let removed = podman::laboratory::image_exists(podman, &reference)
            .await
            .map_err(|e| e.0)?;
        podman::laboratory::image_remove_ignoring(podman, &reference)
            .await
            .map_err(|e| e.0)?;
        let mut caches_removed = 0u32;
        if caches {
            let root = plugin_cache_dir(bin_dir, coords);
            match tokio::fs::read_dir(&root).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        // Best effort per directory. A Containerfile
                        // that switches to a non-root USER before the
                        // caching RUN leaves files owned by a subuid
                        // this process cannot unlink; that must not
                        // fail the reset, whose real job (the image) is
                        // already done.
                        if tokio::fs::remove_dir_all(entry.path()).await.is_ok() {
                            caches_removed += 1;
                        }
                    }
                }
                // Never built, or already cleaned.
                Err(_) => {}
            }
        }
        Ok(
            objectiveai_sdk::laboratories::daemon::PluginImageResetResult {
                removed,
                caches_removed,
            },
        )
    }
    .await;
    claim
        .release()
        .map_err(|e| format!("bin lock release: {e}"))?;
    result
}

/// A registered directory has to be absolute (a bind mount and this
/// host's cwd agree on nothing else), present, and a directory.
/// Checked before any podman call so the error names the registration
/// rather than surfacing as a confusing build failure.
///
/// Public because `host.rs` runs it FIRST, to answer with the
/// development-source error code rather than a generic internal one.
/// [`ensure`] repeats it so it stays correct for any caller — one
/// `metadata` call against a directory we are about to build from.
pub async fn check_development_dir(dir: &Path) -> Result<(), DevelopmentSourceError> {
    if !dir.is_absolute() {
        return Err(DevelopmentSourceError(format!(
            "plugin development directory must be absolute: {}",
            dir.display()
        )));
    }
    match tokio::fs::metadata(dir).await {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(DevelopmentSourceError(format!(
            "plugin development directory {} is not a directory",
            dir.display()
        ))),
        Err(e) => Err(DevelopmentSourceError(format!(
            "plugin development directory {}: {e}",
            dir.display()
        ))),
    }
}
