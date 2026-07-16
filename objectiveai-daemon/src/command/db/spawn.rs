//! Internal db spawn — the daemon starts the `objectiveai-db`
//! postgres supervisor as a leashed resident child when something
//! needs the database (there is no wire `db spawn` command anymore;
//! `Context::db_handle()` is the entry). The supervisor announces the
//! cluster's `postgresql://...` connection URL (postmaster on
//! 127.0.0.1, random free port) over the stdout ready handshake; a
//! live resident child short-circuits to its cached URL.

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::config::DB_DEFAULT_PASSWORD;

/// The spawn flow itself (used by `Context::db_handle()`).
/// Idempotent and cheap when the cluster is already up: a live
/// resident child returns its cached `postgresql://` URL without
/// spawning.
pub async fn spawn(ctx: &Context) -> Result<String, Error> {
    let mut config = ctx
        .filesystem
        .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
        .await?;
    let password = config
        .db()
        .get_password()
        .unwrap_or(DB_DEFAULT_PASSWORD)
        .to_string();

    let bin = if cfg!(windows) {
        "objectiveai-db.exe"
    } else {
        "objectiveai-db"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);

    // objectiveai-db is clap-args-only (no env): the layout
    // coordinates tell it to provision THIS cli's tree — postgres
    // binaries into the shared <dir>/bin/pg-bin, the cluster into
    // <dir>/state/<state>/db.
    let address = crate::spawn::spawn_leashed_until_ready(ctx, "db", &exe, |cmd| {
        cmd.arg("--objectiveai-dir")
            .arg(ctx.filesystem.dir())
            .arg("--objectiveai-state")
            .arg(ctx.filesystem.state())
            .arg("--pg-password")
            .arg(password);
    })
    .await?;
    address.ok_or_else(|| {
        Error::Spawn(
            "objectiveai-db".to_string(),
            std::io::Error::other("db announced ready with no address"),
        )
    })
}
