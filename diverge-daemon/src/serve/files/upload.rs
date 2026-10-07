//! An upload: the files the client sends, each on a channel the
//! daemon opens, landed one by one.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::volumes::upload::server::channel_request;
use diverge_sdk::daemon::reference;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::containers::{Opened, files};
use crate::content;
use crate::daemon::Daemon;
use crate::volumes;

/// Where the upload lands.
pub enum Into<'a> {
    /// A container.
    Opened(&'a Opened),
    /// A volume.
    Volume(&'a reference::Volume),
}

/// What is uploaded: one file at `path`, or the files named at their
/// paths under `path`.
pub enum What<'a> {
    /// One file.
    File,
    /// A directory of these files.
    Directory(&'a [String]),
}

/// Land the upload: each file's content asked for on a channel of
/// its own — the ask naming the file's path for a directory, nothing
/// for a file — and written whole at the destination joined with its
/// path, every parent made, one file after another. A content channel
/// that ends in an error abandons that file and is the error; files
/// landed before it stay. Paths that are not names are the error
/// before anything is asked.
pub async fn upload(scope: Arc<ScopeHandle>, daemon: &Daemon, into: Into<'_>, path: &[String], what: What<'_>) -> Result<(), String> {
    let files: Vec<(Option<String>, Vec<String>)> = match what {
        What::File => {
            if path.is_empty() {
                return Err("a file's path has at least one component".to_string());
            }
            content::inside(path).map_err(|error| error.to_string())?;
            vec![(None, Vec::new())]
        }
        What::Directory(named) => {
            content::inside(path).map_err(|error| error.to_string())?;
            let all = content::validate(named).map_err(|error| error.to_string())?;
            named.iter().zip(all).map(|(name, components)| (Some(name.clone()), components)).collect()
        }
    };
    for (name, components) in files {
        let mut ask = Vec::new();
        channel_request::Frame { path: name }
            .encode(&mut Writer::new(&mut ask))
            .map_err(|error| error.to_string())?;
        let pieces = content::pieces(Arc::clone(&scope), &ask).await;
        let mut destination = path.to_vec();
        destination.extend(components);
        match into {
            Into::Opened(opened) => {
                opened.touch();
                files::write(opened, &destination, pieces).await?;
            }
            Into::Volume(volume) => {
                volumes::write(daemon, volume, &destination, pieces)
                    .await
                    .map_err(|fail| fail.to_string())?;
            }
        }
    }
    Ok(())
}
