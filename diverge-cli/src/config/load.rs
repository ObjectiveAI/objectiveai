//! Reading the file.

use std::io;
use std::path::Path;

use super::{Config, Error};

/// The name the file has, and the only one looked for.
const FILE: &str = "cli.yaml";

/// The configuration at `root`: `cli.yaml` read and parsed, or the
/// built-in default where there is no such file. A file that does not
/// parse, or says something the document does not take, is
/// [`Error::Parse`], with the path to the value that was wrong.
pub async fn load(root: &Path) -> Result<Config, Error> {
    let file = root.join(FILE);
    match tokio::fs::read(&file).await {
        Ok(bytes) => serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_slice(&bytes))
            .map_err(|source| Error::Parse { path: file, source }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(source) => Err(Error::Io { path: file, source }),
    }
}
