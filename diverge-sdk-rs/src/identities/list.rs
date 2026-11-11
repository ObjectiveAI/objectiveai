//! Every identity.

use std::io;
use std::path::Path;

use super::encode::decode;
use super::{Error, Identities, Name};

impl Identities {
    /// Every identity, by issuer and then subject, as the folders and
    /// the files are named: a folder or a `.json` file whose name
    /// the encoding did not make is passed over, and so are the
    /// locks and the staging files. No token is read.
    pub async fn list(&self) -> Result<Vec<Name>, Error> {
        let mut names = Vec::new();
        for (issuer, folder) in entries(&self.folder).await? {
            for (file, _) in entries(&folder).await? {
                let Some(subject) = file.strip_suffix(".json").filter(|stem| !stem.starts_with('.')) else {
                    continue;
                };
                if let (Some(issuer), Some(subject)) = (decode(&issuer), decode(subject)) {
                    names.push(Name { issuer, subject });
                }
            }
        }
        names.sort();
        Ok(names)
    }
}

/// Every entry of `folder`, by name, with its path; none where the
/// folder is absent.
async fn entries(folder: &Path) -> Result<Vec<(String, std::path::PathBuf)>, Error> {
    let failed = |source| Error::Io { path: folder.to_path_buf(), source };
    let mut reader = match tokio::fs::read_dir(folder).await {
        Ok(reader) => reader,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(failed(source)),
    };
    let mut entries = Vec::new();
    while let Some(entry) = reader.next_entry().await.map_err(failed)? {
        if let Some(name) = entry.file_name().to_str() {
            entries.push((name.to_string(), entry.path()));
        }
    }
    Ok(entries)
}
