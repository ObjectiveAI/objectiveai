//! Internal db spawn — the daemon starts the `objectiveai-db`
//! postgres supervisor as a leashed resident child when something
//! needs the database (there is no wire `db spawn` command anymore;
//! `GlobalContext::db_handle()` is the entry). Identity-blind: the db
//! is one shared child, so this flow takes only the [`GlobalContext`]
//! and reads the BOOT filesystem. The supervisor announces the
//! cluster's `postgresql://...` connection URL (postmaster on
//! 127.0.0.1, random free port) over the stdout ready handshake; a
//! live resident child short-circuits to its cached URL.

use crate::context::GlobalContext;
use crate::error::Error;
use crate::filesystem::config::DB_DEFAULT_PASSWORD;

/// The spawn flow itself (used by `GlobalContext::db_handle()`).
/// Idempotent and cheap when the cluster is already up: a live
/// resident child returns its cached `postgresql://` URL without
/// spawning.
pub async fn spawn(global: &GlobalContext) -> Result<String, Error> {
    let mut config = global
        .boot_filesystem()
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
    let exe = global.boot_filesystem().bin_dir().join(bin);

    // objectiveai-db is clap-args-only (no env): the layout
    // coordinates tell it to provision THIS cli's tree — postgres
    // binaries into the shared <dir>/bin/pg-bin, the cluster into
    // <dir>/state/<state>/db.
    let address = crate::spawn::spawn_leashed_until_ready(global, "db", &exe, |cmd| {
        cmd.arg("--objectiveai-dir")
            .arg(global.boot_filesystem().dir())
            .arg("--objectiveai-state")
            .arg(global.boot_filesystem().state())
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
