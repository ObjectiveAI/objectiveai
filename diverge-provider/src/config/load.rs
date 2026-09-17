//! Reading the file, and checking what it says.

use std::collections::HashSet;
use std::io;
use std::path::Path;

use super::Config;
use super::Error;
use crate::volume_manager;

/// The name the file has, and the only one looked for.
const FILE: &str = "config.yaml";

/// The configuration in `dir`: `config.yaml` read and parsed, or the
/// defaults where there is no such file; every path resolved against
/// `dir`; and every rule the sections state checked, so the provider
/// runs on nothing the file got wrong.
///
/// Resolved: `containers.podman.storage_path`, joined onto `dir` when
/// it is relative, and made if absent. Made if absent: every store's
/// directory, since a store is the provider's to fill. Refused: a
/// store or a fixed volume whose path is relative
/// ([`Error::Relative`]); a store whose capacity is `0`
/// ([`Error::Capacity`]); a fixed volume whose path is not an
/// existing directory ([`Error::Missing`]), since a fixed volume is
/// content the operator already has; a fixed volume whose name is
/// not one a volume may have, or is another fixed volume's
/// ([`Error::FixedName`]). A file that does not parse, or names a key
/// no section has, is [`Error::Parse`], with the path to the value
/// that was wrong.
pub async fn load(dir: &Path) -> Result<Config, Error> {
    let file = dir.join(FILE);
    let mut config = match tokio::fs::read(&file).await {
        Ok(bytes) => serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_slice(&bytes)).map_err(
            |source| Error::Parse {
                path: file.clone(),
                source,
            },
        )?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Config::default(),
        Err(source) => return Err(Error::Io { path: file, source }),
    };
    let storage = &mut config.containers.podman.storage_path;
    if !storage.is_absolute() {
        *storage = dir.join(&*storage);
    }
    tokio::fs::create_dir_all(&*storage).await.map_err(|source| Error::Io {
        path: storage.clone(),
        source,
    })?;
    if let Some(volumes) = &config.volumes {
        for store in volumes.stores.iter().flatten() {
            if !store.path.is_absolute() {
                return Err(Error::Relative(store.path.clone()));
            }
            if store.capacity == 0 {
                return Err(Error::Capacity(store.path.clone()));
            }
            tokio::fs::create_dir_all(&store.path).await.map_err(|source| Error::Io {
                path: store.path.clone(),
                source,
            })?;
        }
        let mut names = HashSet::new();
        for fixed in volumes.fixed.iter().flatten() {
            if !fixed.path.is_absolute() {
                return Err(Error::Relative(fixed.path.clone()));
            }
            if !tokio::fs::metadata(&fixed.path).await.is_ok_and(|found| found.is_dir()) {
                return Err(Error::Missing(fixed.path.clone()));
            }
            if !volume_manager::ok(&fixed.name) || !names.insert(fixed.name.as_str()) {
                return Err(Error::FixedName(fixed.name.clone()));
            }
        }
    }
    Ok(config)
}
