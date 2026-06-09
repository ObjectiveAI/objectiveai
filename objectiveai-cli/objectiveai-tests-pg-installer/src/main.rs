//! Test-only helper: extracts the bundled postgres archive into
//! the directory passed as `argv[1]`. The objectiveai-cli test
//! harness runs this once per `test.sh` invocation to populate
//! `<runtime>/_shared-db-bin/`, then symlinks every committed
//! test dir's `db-bin/` to that path. Per-test cli children
//! then see `installation_dir.exists()` ⇒ `pg.setup()` skips
//! the extract step and only runs `initdb` + `pg_ctl start` on
//! the per-test `data_dir`.
//!
//! Does NOT call `pg.start()` — we don't want a daemonized
//! postmaster left running for the harness to chase. The
//! crate's `pg.setup()` does install + initialize; since
//! `initialize()` (initdb) requires a writable data dir, we
//! hand it a `tempfile::tempdir()` that's discarded on Drop.
//! ~5s of wasted initdb work, amortized once per harness run,
//! inside `prepare.sh`'s parallel cargo-build window.

use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let install_dir: PathBuf = std::env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("usage: objectiveai-tests-pg-installer <install-dir>")
    })?.into();

    let scratch = tempfile::tempdir()?;

    let mut settings = postgresql_embedded::Settings::default();
    settings.installation_dir = install_dir;
    settings.data_dir = scratch.path().join("data");
    settings.password_file = scratch.path().join(".pgpass");
    settings.timeout = Some(Duration::from_secs(180));

    let mut pg = postgresql_embedded::PostgreSQL::new(settings);
    pg.setup().await?;

    // No `pg.start()`, no `mem::forget`. `scratch` Drop wipes
    // the throwaway data dir on exit.
    Ok(())
}
