//! Reading a document.

use std::io;
use std::path::Path;

use serde_json::Value;

use super::{Error, Which};

/// The document at `root`, as JSON: the file read and parsed, or the
/// type's built-in default where there is no file. A file that does
/// not parse as YAML is [`Error::Parse`]; nothing here asks whether
/// its type takes it, which is what a `set` or an `edit` asks of what
/// it writes.
pub async fn read(root: &Path, which: Which) -> Result<Value, Error> {
    let file = root.join(which.file());
    match tokio::fs::read(&file).await {
        Ok(bytes) => {
            serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_slice(&bytes))
                .map_err(|source| Error::Parse { path: file, source })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => which.default(),
        Err(source) => Err(Error::Io { path: file, source }),
    }
}
