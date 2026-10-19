//! Extracting the archive once.

use std::path::{Path, PathBuf};
use std::time::Duration;

use diverge_sdk::file_lock;
use postgresql_embedded::{PostgreSQL, Settings};

use super::{Binaries, Error, discard};

/// The file in `bin/<version>/` that says the extraction finished.
const COMPLETE: &str = "complete";

/// How long one extraction, with the throwaway `initdb` that proves
/// it, may take.
const TIMEOUT: Duration = Duration::from_secs(180);

/// How many times an extraction is tried before the start fails.
/// More than once because a virus scanner on Windows can hold a file
/// just written long enough for `initdb` to fail on it.
const ATTEMPTS: u32 = 3;

/// The binaries in `bin`, extracted now if they were not already.
///
/// `bin/<version>/complete` present is the fast path, and nothing is
/// locked. Otherwise `bin/locks/install.lock` is taken, waiting for
/// any other start that holds it, the marker is looked for again —
/// the other start may have finished the work — and if it is still
/// absent whatever is at `bin/<version>/` is thrown away and the
/// archive extracted there, proven by a throwaway `initdb` into a
/// scratch directory that is deleted afterwards, and the marker
/// written. The lock is let go when this returns.
pub async fn ensure(bin: &Path) -> Result<Binaries, Error> {
    let locks = bin.join("locks");
    tokio::fs::create_dir_all(&locks).await.map_err(|source| Error::Io {
        path: locks.clone(),
        source,
    })?;
    let home = installed_home(bin);
    let marker = home.join(COMPLETE);
    if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        return Ok(Binaries::new(home));
    }
    let _turn = file_lock::wait_exclusive(locks.join("install.lock")).await.map_err(Error::Lock)?;
    if tokio::fs::try_exists(&marker).await.unwrap_or(false) {
        return Ok(Binaries::new(home));
    }
    let mut attempt = 1;
    loop {
        discard(&home).await.map_err(|source| Error::Io {
            path: home.clone(),
            source,
        })?;
        match extract(bin).await {
            Ok(()) => break,
            Err(error) if attempt < ATTEMPTS => {
                let _ = error;
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(error) => return Err(error),
        }
    }
    tokio::fs::write(&marker, b"").await.map_err(|source| Error::Io {
        path: marker,
        source,
    })?;
    Ok(Binaries::new(home))
}

/// `bin/<version>/` for the version baked in: `postgresql_embedded`
/// appends the exact version to the installation directory it is
/// given, and this reads it back from the settings it built.
fn installed_home(bin: &Path) -> PathBuf {
    PostgreSQL::new(scratch_settings(bin, Path::new(""))).settings().installation_dir.clone()
}

/// Settings for an extraction into `bin` with a throwaway cluster in
/// `scratch`.
///
/// `Settings::new` is the one way to learn the baked-in version, and
/// it makes two temporary directories of its own as it goes; both are
/// removed here, since the scratch directory replaces them.
fn scratch_settings(bin: &Path, scratch: &Path) -> Settings {
    let mut settings = Settings::new();
    let _ = std::fs::remove_dir(&settings.data_dir);
    if let Some(made) = settings.password_file.parent() {
        let _ = std::fs::remove_dir(made);
    }
    settings.installation_dir = bin.to_path_buf();
    settings.data_dir = scratch.join("data");
    settings.password_file = scratch.join("password");
    settings.host = "127.0.0.1".to_string();
    settings.password = "scratch".to_string();
    settings.temporary = true;
    settings.timeout = Some(TIMEOUT);
    settings
}

/// One extraction into `bin`, proven by a throwaway `initdb` into a
/// scratch directory under the system's temporary directory, removed
/// afterwards whatever happened.
async fn extract(bin: &Path) -> Result<(), Error> {
    let scratch = std::env::temp_dir().join(format!("diverge-postgres-scratch-{}", std::process::id()));
    tokio::fs::create_dir_all(&scratch).await.map_err(|source| Error::Io {
        path: scratch.clone(),
        source,
    })?;
    let outcome = {
        let mut postgres = PostgreSQL::new(scratch_settings(bin, &scratch));
        postgres.setup().await
    };
    let _ = tokio::fs::remove_dir_all(&scratch).await;
    outcome.map_err(Error::Extract)
}
