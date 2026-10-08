//! Reading the file, and resolving what it names.

use std::io;
use std::path::{Path, PathBuf};

use super::Config;
use super::Error;

/// The name the file has, and the only one looked for.
const FILE: &str = "config.yaml";

/// The configuration at `root`: `config.yaml` read and parsed, or the
/// defaults where there is no such file, with every path in it
/// resolved against `root` — `containers.podman.storage_path`, each
/// store's and each fixed volume's `path` of the provider's block —
/// so that what comes back holds every path absolute. A file that
/// does not parse, or names a key no block has, is [`Error::Parse`],
/// with the path to the value that was wrong. Nothing on disk is
/// made or checked here: that is each program's, at its own start.
pub async fn load(root: &Path) -> Result<Config, Error> {
    let file = root.join(FILE);
    let mut config: Config = match tokio::fs::read(&file).await {
        Ok(bytes) => serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_slice(&bytes))
            .map_err(|source| Error::Parse { path: file, source })?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Config::default(),
        Err(source) => return Err(Error::Io { path: file, source }),
    };
    resolve(root, &mut config.provider.containers.podman.storage_path);
    if let Some(volumes) = &mut config.provider.volumes {
        for store in volumes.stores.iter_mut().flatten() {
            resolve(root, &mut store.path);
        }
        for fixed in volumes.fixed.iter_mut().flatten() {
            resolve(root, &mut fixed.path);
        }
    }
    Ok(config)
}

/// A relative path joined onto the root; an absolute one as written.
fn resolve(root: &Path, path: &mut PathBuf) {
    if !path.is_absolute() {
        *path = root.join(&*path);
    }
}
