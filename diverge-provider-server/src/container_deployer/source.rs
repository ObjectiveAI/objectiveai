//! Where an image was found, as the reference podman is handed; and
//! the finding.

use std::path::Path;

use diverge_sdk::provider::server::caller::Caller;
use futures_util::StreamExt as _;
use futures_util::future::Either;
use futures_util::stream::FuturesUnordered;

use super::{ContainerDeployer, Error};
use crate::tools::podman;

/// Where the image of a deploy was found: the place, resolved to the
/// reference podman is handed and whether it is pulled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// With the caller, served by the provider's own registry:
    /// `127.0.0.1:<port>/<repository>/<name>@<digest>`, pulled over
    /// plain HTTP.
    Client(String),
    /// In the store, as one of the provider's own: `<name>@<digest>`,
    /// pulled from nowhere.
    Server(String),
    /// In a registry the configuration lists:
    /// `<host>/<name>@<digest>`, pulled with that host's credential.
    Registry(String),
}

/// Where the image is, among the places the provider looks all at
/// once: every registry the configuration lists, asked with the
/// credential listed, and the caller, asked on its scope. The first
/// to answer that it holds the pair is the source, and the rest are
/// not waited for — a look into a registry still running is a podman
/// dropped and killed. No yes from any of them is
/// [`Error::Unavailable`].
pub(super) async fn find(deployer: &ContainerDeployer, name: &str, digest: &str, caller: &Caller) -> Result<Source, Error> {
    let mut asked = FuturesUnordered::new();
    for registry in &deployer.podman.registries {
        asked.push(Either::Left(in_registry(deployer.auth_file(), &registry.host, name, digest)));
    }
    asked.push(Either::Right(with_caller(deployer, caller, name, digest)));
    while let Some(found) = asked.next().await {
        if let Some(source) = found? {
            return Ok(source);
        }
    }
    Err(Error::Unavailable {
        name: name.to_string(),
        digest: digest.to_string(),
    })
}

/// One registry asked whether it serves the pair.
async fn in_registry(auth_file: &Path, host: &str, name: &str, digest: &str) -> Result<Option<Source>, Error> {
    let reference = format!("{host}/{name}@{digest}");
    let found = podman::manifest_exists(auth_file, &reference).await.map_err(Error::Podman)?;
    Ok(found.then_some(Source::Registry(reference)))
}

/// The caller asked whether it holds the pair; when it does, the
/// source is the provider's own registry at the port podman reaches
/// it by, serving this run's repository.
async fn with_caller(deployer: &ContainerDeployer, caller: &Caller, name: &str, digest: &str) -> Result<Option<Source>, Error> {
    if !caller.holds().await {
        return Ok(None);
    }
    let port = deployer.registry_port(caller.registry());
    Ok(Some(Source::Client(format!("127.0.0.1:{port}/{}/{name}@{digest}", caller.repository()))))
}

impl Source {
    /// The reference podman is handed.
    pub fn reference(&self) -> &str {
        match self {
            Source::Client(reference) | Source::Server(reference) | Source::Registry(reference) => reference,
        }
    }

    /// Whether the image is pulled before the run.
    pub fn pulled(&self) -> bool {
        !matches!(self, Source::Server(_))
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
