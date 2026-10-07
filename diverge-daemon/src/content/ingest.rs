//! Files, from wherever they come, taken in and hashed.

use std::path::Path;

use diverge_sdk::daemon::endpoints::resources::Kind;
use sha2::{Digest as _, Sha256};
use tokio::io::AsyncWriteExt as _;
use tokio::task::JoinSet;

use super::{Error, Pieces, Received, hash_of, paths};
use crate::store::hash;

/// Take the files in: each stream written under a fresh directory in
/// `incoming` at its path — the one file of a file resource at
/// `file` — and hashed as it lands, every one on a task of its own.
/// The id is the one file's hash, or the directory's `dirhash` over
/// every file's; a stream that ends in an error abandons the whole,
/// and the directory is removed.
pub async fn ingest(incoming: &Path, kind: Kind, files: Vec<(Vec<String>, Pieces)>) -> Result<Received, Error> {
    let fresh = incoming.join(uuid::Uuid::new_v4().to_string());
    let outcome = ingest_into(&fresh, kind, files).await;
    if outcome.is_err() {
        let _ = tokio::fs::remove_dir_all(&fresh).await;
    }
    outcome
}

/// The ingest into `fresh`, which does not exist yet.
async fn ingest_into(fresh: &Path, kind: Kind, files: Vec<(Vec<String>, Pieces)>) -> Result<Received, Error> {
    tokio::fs::create_dir_all(fresh).await.map_err(|source| Error::Io {
        path: fresh.to_path_buf(),
        source,
    })?;
    match kind {
        Kind::File => {
            let Some((_, pieces)) = files.into_iter().next() else {
                return Err(Error::Abandoned(String::new()));
            };
            let at = fresh.join("file");
            let (digest, bytes) = landed(pieces, at.clone()).await?;
            Ok(Received {
                incoming: at,
                kind: Kind::File,
                id: hash::file_id(&digest),
                bytes,
            })
        }
        Kind::Directory => {
            let mut landing = JoinSet::new();
            for (components, pieces) in files {
                let at = paths::join(fresh, &components);
                if let Some(parent) = at.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|source| Error::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
                let name = components.join("/");
                landing.spawn(async move { landed(pieces, at).await.map(|(digest, bytes)| (name, digest, bytes)) });
            }
            let mut hashed = Vec::new();
            let mut total = 0;
            while let Some(joined) = landing.join_next().await {
                let (name, digest, bytes) = joined.map_err(|_| Error::Abandoned(String::new()))??;
                total += bytes;
                hashed.push((name, hash::file_id(&digest)));
            }
            Ok(Received {
                incoming: fresh.to_path_buf(),
                kind: Kind::Directory,
                id: hash::directory_id(&hashed),
                bytes: total,
            })
        }
    }
}

/// One file's pieces written to `at` and hashed, to the end.
async fn landed(mut pieces: Pieces, at: std::path::PathBuf) -> Result<(Vec<u8>, u64), Error> {
    use futures_util::StreamExt as _;
    let mut file = tokio::fs::File::create(&at).await.map_err(|source| Error::Io {
        path: at.clone(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut bytes = 0;
    while let Some(piece) = pieces.next().await {
        let piece = piece.map_err(Error::Abandoned)?;
        file.write_all(&piece).await.map_err(|source| Error::Io {
            path: at.clone(),
            source,
        })?;
        hasher.update(&piece);
        bytes += piece.len() as u64;
    }
    file.flush().await.map_err(|source| Error::Io { path: at, source })?;
    Ok((hash_of(hasher), bytes))
}
