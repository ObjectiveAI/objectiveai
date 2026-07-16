//! `laboratories spawn` — start the machine's resident laboratory HOST
//! in the background.
//!
//! The host is per-state: its lock lives at `<dir>/state/<state>/locks`
//! key `laboratories` — ONE lock however many daemon connections the
//! host keeps. The host is a WebSocket client (no listener), so the
//! lock content is a plain readiness marker, not a URL.
//!
//! The dial list comes from config: unless `laboratories config local`
//! is false, the LOCAL daemon is ensured and dialed first (with the
//! signature from the DAEMON's own config — bare `SIGNATURE` env, else
//! derived from `SECRET`); then every `laboratories config addresses`
//! entry, each with its own optional signature. Everything rides argv
//! (`--address` repeated + `--signature ADDRESS=SIGNATURE` repeated) —
//! the host binary reads NO environment variables, by design.

use objectiveai_sdk::cli::command::laboratories::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

/// How long a LOCAL spawn waits for the host to appear in the daemon
/// registry. Generous: podman (and its machine VM) may be cold.
const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// The spawn flow itself, callable in-process (used by `create` /
/// `delete` auto-spawns as well as the `laboratories spawn` command).
/// Idempotent and cheap when the host is already up: a try_read of the
/// lock returns without spawning. Returns every address the host was
/// told to dial.
pub async fn spawn(ctx: &Context) -> Result<Vec<String>, Error> {
    let config = ctx
        .filesystem
        .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
        .await?
        .laboratories
        .unwrap_or_default();
    let local = config.local != Some(false);

    let mut addresses: Vec<String> = Vec::new();
    let mut signatures: Vec<String> = Vec::new();
    if local {
        // Ensure the local daemon (idempotent) and dial it FIRST, with
        // the signature from the daemon's OWN config — never from the
        // addresses map.
        let daemon_address = crate::command::daemon::spawn::spawn(ctx).await?;
        if let Some(signature) = ctx.config.client_signature() {
            signatures.push(format!("{daemon_address}={signature}"));
        }
        addresses.push(daemon_address);
    }
    for (address, signature) in config.addresses.unwrap_or_default() {
        // The local daemon may also be a configured entry — one
        // connection per address, local signature wins.
        if addresses.contains(&address) {
            continue;
        }
        if !signature.is_empty() {
            signatures.push(format!("{address}={signature}"));
        }
        addresses.push(address);
    }
    if addresses.is_empty() {
        return Err(Error::Laboratory(
            "laboratories config local is false and no addresses are configured — \
             the host would have nothing to dial"
                .to_string(),
        ));
    }

    let exe = ctx.filesystem.bin_dir().join(if cfg!(windows) {
        "objectiveai-laboratory.exe"
    } else {
        "objectiveai-laboratory"
    });
    let lock_dir = ctx.filesystem.state_dir().join("locks");
    crate::spawn::spawn_until_lock_published(&exe, &lock_dir, "laboratories", |cmd| {
        // No subcommand — the binary IS the host; bare args only.
        for address in &addresses {
            cmd.arg("--address").arg(address);
        }
        for signature in &signatures {
            cmd.arg("--signature").arg(signature);
        }
        cmd.arg("--objectiveai-dir")
            .arg(ctx.filesystem.dir())
            .arg("--objectiveai-state")
            .arg(ctx.filesystem.state())
            .arg("--suppress-output");
    })
    .await?;

    // Readiness. LOCAL: connected = this machine's host visible in the
    // daemon registry — poll it, failing fast if the host dies (its
    // lock releases). Remote-only: this machine cannot see the remote
    // registries; lock publication is the whole contract (the host
    // retries its dials forever).
    if local {
        let machine_id =
            objectiveai_sdk::machine::machine_id(ctx.filesystem.dir());
        let deadline = std::time::Instant::now() + READY_TIMEOUT;
        loop {
            // Readiness = OUR host — the exact (machine, OWN state)
            // pair; a same-machine host of another state is somebody
            // else's.
            if let Some(hubs) = ctx.resident_hubs()
                && hubs
                    .laboratories
                    .has_host(&machine_id, ctx.filesystem.state())
            {
                break;
            }
            if !objectiveai_sdk::lockfile::try_held(&lock_dir, "laboratories").await {
                return Err(Error::Laboratory(
                    "the laboratory host exited before connecting to the daemon"
                        .to_string(),
                ));
            }
            if std::time::Instant::now() >= deadline {
                return Err(Error::Laboratory(format!(
                    "the laboratory host did not connect to the daemon within {}s",
                    READY_TIMEOUT.as_secs()
                )));
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }

    Ok(addresses)
}

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    Ok(Response {
        addresses: spawn(ctx).await?,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::spawn as sdk;
    use objectiveai_sdk::cli::command::laboratories::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::spawn as sdk;
    use objectiveai_sdk::cli::command::laboratories::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
