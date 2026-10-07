//! Downloading a resource, or a part of a directory one.

use std::path::Path;

use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::daemon::download::Chunk;
use diverge_sdk::daemon::endpoints::resources::download::client::request;
use diverge_sdk::daemon::endpoints::resources::download::server::response::Frame;
use diverge_sdk::daemon::grant::resources::Over;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::io::AsyncReadExt as _;

use super::in_use;
use crate::content;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, resources};

/// Send what is at the path and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// `Forbidden` with no `download` grant at all; `NotFound` for a
/// resource not held, or a path at which nothing is; `Forbidden` for a
/// resource the grants do not reach; else every file at the path, one
/// chunk at a time with the file's path relative to what was asked —
/// the empty path for a file — files in bytewise order of their paths,
/// a zero-byte file one empty chunk, and a directory with no file
/// nothing at all.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    if !judge::resources::holds(&standing, Over::Download) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    let Some(record) = resources::by_id(&mut conn, &frame.resource, false).await? else {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    };
    drop(conn);
    if !judge::resources::over(&standing, Over::Download, &record, in_use(&record)) {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    if content::inside(&frame.path).is_err() {
        reply::reply(scope, &Frame::NotFound).await;
        return Ok(());
    }
    let root = content::held(&daemon.resources, &record.id);
    let files = match content::at(&root, &frame.path).await {
        None => {
            reply::reply(scope, &Frame::NotFound).await;
            return Ok(());
        }
        Some(content::Found::File(path)) => vec![(Vec::new(), path)],
        Some(content::Found::Directory(dir)) => match content::files(&dir).await {
            Ok(files) => files,
            Err(error) => {
                reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
                return Ok(());
            }
        },
    };
    for (relative, path) in files {
        if let Err(error) = send_file(scope, &relative, &path).await {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    }
    Ok(())
}

/// One file as chunks of at most `CHUNK_SIZE`, each carrying
/// `relative`; a zero-byte file as one empty chunk.
async fn send_file(scope: &ScopeHandle, relative: &[String], path: &Path) -> Result<(), content::Error> {
    let mut file = tokio::fs::File::open(path).await.map_err(|source| content::Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut sent_any = false;
    loop {
        let read = file.read(&mut buffer).await.map_err(|source| content::Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        reply::reply(
            scope,
            &Frame::Chunk(Chunk {
                path: relative.to_vec(),
                body: &buffer[..read],
            }),
        )
        .await;
        sent_any = true;
    }
    if !sent_any {
        reply::reply(
            scope,
            &Frame::Chunk(Chunk {
                path: relative.to_vec(),
                body: &[],
            }),
        )
        .await;
    }
    Ok(())
}
