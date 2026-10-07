//! Reading the file.

use std::io;
use std::path::Path;

use super::Config;
use super::Error;

/// The name the file has, and the only one looked for.
const FILE: &str = "config.yaml";

/// The configuration in `dir`: `config.yaml` read and parsed, or the
/// defaults where there is no such file. A file that does not parse,
/// or names a key the document does not have, is [`Error::Parse`],
/// with the path to the value that was wrong.
pub async fn load(dir: &Path) -> Result<Config, Error> {
    let file = dir.join(FILE);
    match tokio::fs::read(&file).await {
        Ok(bytes) => serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_slice(&bytes))
            .map_err(|source| Error::Parse { path: file, source }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(source) => Err(Error::Io { path: file, source }),
    }
}
