//! Making the cluster, once.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use super::{Error, PASSWORD};
use crate::install::{self, Binaries};

/// The data directory's name in the directory.
pub const DATA: &str = "data";

/// The marker beside it that says `initdb` finished.
const READY: &str = "data.ready";

/// How long `initdb` may take. It routinely takes ten to thirty
/// seconds on a slow disk, and much longer under a virus scanner.
const TIMEOUT: Duration = Duration::from_secs(180);

/// The cluster at `<dir>/data`, made now if `<dir>/data.ready` is
/// absent: a data directory without the marker is thrown away as a
/// partial, and `initdb` is run into a fresh one as the superuser
/// `postgres` with the password in `<dir>/password`, `scram-sha-256`
/// for every connection, and UTF-8; the marker is written when it
/// exits well. Nothing is locked here: the caller holds the init
/// lock around this, the stop and the start together.
pub async fn init(dir: &Path, binaries: &Binaries) -> Result<(), Error> {
    let data = dir.join(DATA);
    let ready = dir.join(READY);
    if tokio::fs::try_exists(&ready).await.unwrap_or(false) {
        return Ok(());
    }
    install::discard(&data).await.map_err(|source| Error::Io {
        path: data.clone(),
        source,
    })?;
    let mut initdb = Command::new(binaries.initdb());
    initdb
        .arg("--pgdata")
        .arg(&data)
        .arg("--username")
        .arg("postgres")
        .arg("--auth")
        .arg("scram-sha-256")
        .arg("--pwfile")
        .arg(dir.join(PASSWORD))
        .arg("--encoding")
        .arg("UTF8")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(TIMEOUT, initdb.output())
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(Error::Spawn)?;
    if !output.status.success() {
        return Err(Error::Initdb {
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    tokio::fs::write(&ready, b"").await.map_err(|source| Error::Io { path: ready, source })
}
