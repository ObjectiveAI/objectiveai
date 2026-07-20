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
/// Image label carrying the git commit SHA the image built from.
const SHA_LABEL: &str = "objectiveai.plugin.sha";

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

    /// The derived laboratory id.
    pub fn laboratory_id(&self) -> String {
        format!(
            "{}{}-{}-{}",
            objectiveai_sdk::laboratories::PLUGIN_LABORATORY_ID_PREFIX,
            self.owner,
            self.name,
            self.version,
        )
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
    pub sha: Option<String>,
}

/// Ensure the plugin's image exists locally, building it if needed.
///
/// - Fast path: `podman image exists` → read port/SHA off the image
///   labels, NO clone.
/// - Build path: take the machine-wide bin lock, RE-CHECK existence
///   (a sibling host may have finished the build while we waited),
///   then fetch the repo at the version's git tag, read the manifest,
///   `podman build` (context = checkout root) with the port/SHA
///   stamped as labels, delete the checkout (success AND failure),
///   and release the lock on EVERY path — a `LockClaim` drop
///   deliberately does NOT release (`podman/install.rs` pattern).
pub async fn ensure(
    podman: &Podman,
    bin_dir: &Path,
    coords: &PluginCoords,
) -> Result<EnsuredPluginImage, String> {
    let reference = coords.image_reference();
    if podman::laboratory::image_exists(podman, &reference)
        .await
        .map_err(|e| e.0)?
    {
        return from_labels(podman, &reference).await;
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
        if podman::laboratory::image_exists(podman, &reference)
            .await
            .map_err(|e| e.0)?
        {
            return from_labels(podman, &reference).await;
        }
        let checkout = crate::gitrepo::fetch_at_tag(
            bin_dir,
            &coords.owner,
            &coords.name,
            coords.git_tag(),
        )
        .await?;
        let built = async {
            let (manifest, containerfile) =
                crate::plugin_manifest::read(&checkout.dir).await?;
            let labels = vec![
                (PORT_LABEL.to_string(), manifest.port.to_string()),
                (SHA_LABEL.to_string(), checkout.commit_sha.clone()),
            ];
            podman::laboratory::image_build(
                podman,
                &containerfile,
                &checkout.dir,
                &reference,
                &labels,
            )
            .await
            .map_err(|e| e.0)?;
            Ok(EnsuredPluginImage {
                port: manifest.port,
                sha: Some(checkout.commit_sha.clone()),
            })
        }
        .await;
        // The checkout is transient scratch — gone the moment the
        // build concludes, success or failure.
        crate::gitrepo::remove_checkout(&checkout.dir).await;
        built
    }
    .await;
    claim
        .release()
        .map_err(|e| format!("bin lock release: {e}"))?;
    result
}

/// The exists-fast-path metadata read: the port label is REQUIRED (an
/// image under our tag without it was not built by this flow — refuse
/// rather than guess a port), the SHA label advisory.
async fn from_labels(
    podman: &Podman,
    reference: &str,
) -> Result<EnsuredPluginImage, String> {
    let port = podman::laboratory::image_label(podman, reference, PORT_LABEL)
        .await
        .map_err(|e| e.0)?
        .ok_or_else(|| {
            format!("image {reference} is missing the {PORT_LABEL} label")
        })?;
    let port: u16 = port
        .parse()
        .map_err(|e| format!("image {reference} {PORT_LABEL} label: {e}"))?;
    if port == 0 {
        return Err(format!("image {reference} {PORT_LABEL} label is 0"));
    }
    let sha = podman::laboratory::image_label(podman, reference, SHA_LABEL)
        .await
        .map_err(|e| e.0)?;
    Ok(EnsuredPluginImage { port, sha })
}
