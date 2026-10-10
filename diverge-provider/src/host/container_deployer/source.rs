//! Where an image was found, as the reference podman is handed; and
//! the finding.

use std::path::Path;

use crate::protocol::caller::Caller;
use futures_util::StreamExt as _;
use futures_util::future::Either;
use futures_util::stream::FuturesUnordered;

use super::{ContainerDeployer, Error};
use crate::host::tools::podman;
use diverge_sdk::shared::containers::request::Image;

/// Where the image of a deploy was found: the place, resolved to the
/// reference podman is handed and whether it is pulled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// With the caller, served by the provider's own registry:
    /// `127.0.0.1:<port>/<repository>/<name>@<digest>`, with `name`
    /// the path the caller answered, pulled over plain HTTP.
    Client(String),
    /// In the store already, found by digest: its id, run from the
    /// store, pulled from nowhere.
    Local(String),
    /// In a referenced registry the configuration lists:
    /// `<registry>/<name>@<digest>` as the reference names it, pulled
    /// with that registry's credential.
    Registry(String),
}

/// Where the image is, once the store has not got it, among the
/// places the provider looks all at once: every referenced registry
/// the configuration lists, asked as `<registry>/<name>@<digest>` with
/// the credential listed — a reference naming a registry not listed
/// is ignored — and the caller, asked on its scope. The first to
/// answer that it holds the image is the source, and the rest are not
/// waited for — a look into a registry still running is a podman
/// dropped and killed. No yes from any of them is
/// [`Error::Unavailable`].
pub(super) async fn find(deployer: &ContainerDeployer, image: &Image, caller: &Caller) -> Result<Source, Error> {
    let mut asked = FuturesUnordered::new();
    for reference in &image.references {
        if deployer.podman.registries.iter().any(|registry| registry.host == reference.registry) {
            asked.push(Either::Left(in_registry(deployer.auth_file(), &reference.registry, &reference.name, &image.digest)));
        }
    }
    asked.push(Either::Right(with_caller(deployer, caller, &image.digest)));
    while let Some(found) = asked.next().await {
        if let Some(source) = found? {
            return Ok(source);
        }
    }
    Err(Error::Unavailable {
        digest: image.digest.clone(),
    })
}

/// One referenced registry asked whether it serves the image under
/// the reference's name.
async fn in_registry(auth_file: &Path, host: &str, name: &str, digest: &str) -> Result<Option<Source>, Error> {
    let reference = format!("{host}/{name}@{digest}");
    let found = podman::manifest_exists(auth_file, &reference).await.map_err(Error::Podman)?;
    Ok(found.then_some(Source::Registry(reference)))
}

/// The caller asked whether it holds the image; when it does, under
/// the repository path it answered, the source is the provider's own
/// registry at the port podman reaches it by, serving this run's
/// repository. A path answered that is not a repository path is taken
/// as not held, since it lands in the reference by concatenation.
async fn with_caller(deployer: &ContainerDeployer, caller: &Caller, digest: &str) -> Result<Option<Source>, Error> {
    let Some(name) = caller.holds().await.filter(|name| name_ok(name)) else {
        return Ok(None);
    };
    let port = deployer.registry_port(caller.registry());
    Ok(Some(Source::Client(format!("127.0.0.1:{port}/{}/{name}@{digest}", caller.repository()))))
}

impl Source {
    /// The reference podman is handed.
    pub fn reference(&self) -> &str {
        match self {
            Source::Client(reference) | Source::Local(reference) | Source::Registry(reference) => reference,
        }
    }

    /// Whether the image is pulled before the run.
    pub fn pulled(&self) -> bool {
        !matches!(self, Source::Local(_))
    }

    /// Whether the pull is from the provider's own registry, which
    /// speaks plain HTTP.
    pub fn own(&self) -> bool {
        matches!(self, Source::Client(_))
    }
}

/// Whether `name` is a repository path as the OCI distribution
/// specification has one: one or more segments joined by `/`, each
/// lowercase letters and digits with single `.`, `_`, `__` or `-`
/// runs between them. Anything else — an empty segment, `..`, an
/// uppercase letter, a space — is refused, since the name is put
/// into a URL by concatenation and never normalized.
pub fn name_ok(name: &str) -> bool {
    !name.is_empty() && name.split('/').all(segment_ok)
}

/// One segment of a repository path: names joined by separators,
/// where a name is lowercase letters and digits and a separator is
/// `.`, `_`, `__` or a run of `-`, never first, never last, never
/// two in a row.
fn segment_ok(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    let name = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    let mut index = 0;
    while index < bytes.len() {
        if name(bytes[index]) {
            index += 1;
            continue;
        }
        if index == 0 {
            return false;
        }
        match bytes[index] {
            b'.' => index += 1,
            b'_' => {
                index += 1;
                if bytes.get(index) == Some(&b'_') {
                    index += 1;
                }
            }
            b'-' => {
                while bytes.get(index) == Some(&b'-') {
                    index += 1;
                }
            }
            _ => return false,
        }
        if !bytes.get(index).is_some_and(|&byte| name(byte)) {
            return false;
        }
    }
    !bytes.is_empty()
}
