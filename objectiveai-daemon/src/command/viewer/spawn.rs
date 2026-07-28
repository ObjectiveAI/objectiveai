//! `viewer spawn` — start the `objectiveai-viewer` Tauri shell as a
//! leashed resident child (key `viewer`; a live child makes respawn a
//! no-op). The viewer is an SSE CLIENT of the daemon's broadcast (not
//! a server), so its ready line carries no address.
//!
//! The viewer's whole daemon-facing input is frozen at spawn — env
//! (`DAEMON_ADDRESS`, the daemon's LIVE published connect URL;
//! `DAEMON_SIGNATURE`, the client signature its auth validates) and
//! argv (`--development-plugin` entries, one per viewer development
//! registration). Every mutation of either — `daemon config set`,
//! `refresh-secret-signature-pair`, `development plugins viewer
//! create`/`delete`, `development viewer set`/`delete` — propagates
//! through ONE mechanism: `respawn_running_viewer`. No live channel,
//! nothing to converge.
//!
//! Two spawn forms, chosen by the `development viewer` slot: the
//! installed binary (default), or `pnpm exec tauri dev` in a source
//! checkout (development) — a process TREE, spawned with tree-kill
//! semantics.

use objectiveai_sdk::cli::command::viewer::spawn::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// Quote one token for a cmd.exe command line: wrap in double quotes
/// when it contains whitespace (registration paths), doubling any
/// embedded quote. The trio segments and flag names never need it.
#[cfg(windows)]
fn quote(token: &str) -> String {
    if token.contains(char::is_whitespace) || token.contains('"') {
        format!("\"{}\"", token.replace('"', "\"\""))
    } else {
        token.to_string()
    }
}

/// Unix: tokens are passed as real argv, no quoting layer exists.
#[cfg(unix)]
fn quote(token: &str) -> String {
    token.to_string()
}

/// The spawn flow itself. Idempotent and cheap when the viewer is
/// already up: a try_read of the lock returns without spawning.
pub async fn spawn(global: &GlobalContext, scoped: &ScopedContext) -> Result<String, Error> {
    // The viewer requires the daemon's http:// connect URL. `run`'s
    // producer tee ensured the daemon and recorded the address on the
    // global context before this handler ran; if it's absent the daemon couldn't
    // be spawned, and a viewer without a daemon is useless — error out.
    let daemon_address = global
        .daemon_address()
        .ok_or(Error::DaemonAddressUnavailable)?
        .to_string();

    let bin = if cfg!(windows) {
        "objectiveai-viewer.exe"
    } else {
        "objectiveai-viewer"
    };
    let exe = scoped.filesystem.bin_dir().join(bin);

    // The daemon auth signature: the daemon's own bare `SIGNATURE`
    // env when set, else derived one-way from its bare `SECRET`
    // (`sha256=<hex(SHA256(secret))>`, the same math as
    // `generate_viewer_secret_signature_pair`). Clients send it
    // verbatim in the `X-OBJECTIVEAI-SIGNATURE` header on every daemon
    // HTTP request (the `/laboratory` WebSocket keeps the `AuthEnvelope` preamble).
    let daemon_signature = global.client_signature();

    // The child inherits the cli's environment; every env key the
    // viewer's config reads (`EnvConfigBuilder` in
    // `objectiveai-viewer/src-tauri/src/run.rs`: DAEMON_ADDRESS,
    // DAEMON_SIGNATURE, SUPPRESS_OUTPUT, OBJECTIVEAI_DIR,
    // OBJECTIVEAI_STATE) is set explicitly here when known.
    // `DAEMON_ADDRESS` is the daemon's full http:// connect URL the viewer
    // (a client) dials (always set — required above). `DAEMON_SIGNATURE`
    // is derived here from the daemon's own bare `SECRET` when it has one;
    // otherwise any inherited `DAEMON_SIGNATURE` is left as-is (the spawner
    // may know the signature without the secret). The daemon's own bind
    // config lives in the bare `ADDRESS`/`PORT`/`SECRET` namespace,
    // distinct from these client-facing `DAEMON_` vars.
    // The development-plugin registrations ride the ARGV, read fresh
    // from the registry at every spawn — which is the entire
    // propagation story: `development plugins viewer create`/`delete`
    // respawn a running viewer, and an absent one picks the current
    // set up here whenever it is next spawned. A viewer binary built
    // without its `development` feature ignores argv entirely, which
    // degrades to a viewer without dev mode, nothing worse.
    let development: Vec<String> = global
        .resident_hubs()
        .map(|hubs| {
            hubs.development_plugins
                .viewer
                .list()
                .into_iter()
                .map(|((owner, name, version), path)| {
                    // `<owner>/<name>/<version>=<path>` — the trio's
                    // charset excludes both separators, so the FIRST
                    // `=` split viewer-side is unambiguous even for
                    // paths containing one.
                    format!("{owner}/{name}/{version}={}", path.display())
                })
                .collect()
        })
        .unwrap_or_default();

    let env = |cmd: &mut tokio::process::Command| {
        cmd.env("OBJECTIVEAI_DIR", scoped.filesystem.dir())
            .env("OBJECTIVEAI_STATE", scoped.filesystem.state())
            .env("SUPPRESS_OUTPUT", "true")
            .env("DAEMON_ADDRESS", &daemon_address);
        if let Some(signature) = &daemon_signature {
            cmd.env("DAEMON_SIGNATURE", signature);
        }
    };

    // VIEWER DEVELOPMENT MODE (`development viewer set`): run the
    // viewer FROM SOURCE — `pnpm exec tauri dev` in the registered
    // directory — instead of the installed binary. Same leash key,
    // same ready handshake (run.rs prints the ready line in every
    // mode), same env handoff. The `development` cargo feature makes
    // the source build parse the plugin argv, and tauri dev's `--`
    // passthrough delivers it — and re-delivers it verbatim when
    // tauri dev restarts the binary on its own Rust rebuilds.
    //
    // A source build that fails to start (cargo or vite error) makes
    // the child exit before the ready line, which the spawn machinery
    // reports as an error CARRYING THE BUILD OUTPUT — a failed start
    // is always the caller's error, never a silent hang.
    //
    // TREE spawn: cmd → pnpm → node → cargo → viewer is a process
    // tree, so kills must take the whole tree (KillStyle::Tree).
    if let Some(dir) = global
        .resident_hubs()
        .and_then(|hubs| hubs.development_plugins.viewer_app.get())
    {
        let mut passthrough = String::new();
        for entry in &development {
            passthrough.push_str(&format!(" --development-plugin {}", quote(entry)));
        }
        #[cfg(windows)]
        let (program, configure): (&str, Box<dyn FnOnce(&mut tokio::process::Command) + Send>) = {
            // `pnpm` is a `.cmd` shim, which Rust will not spawn
            // directly — go through cmd.exe, composing the ONE quoted
            // line ourselves via raw_arg (std's auto-quoting and
            // cmd.exe's re-parsing disagree; hand-quoting each token
            // with spaces is the reliable intersection).
            let line = format!(
                "/C pnpm exec tauri dev --features development --{passthrough}"
            );
            ("cmd", Box::new(move |cmd: &mut tokio::process::Command| {
                use std::os::windows::process::CommandExt as _;
                cmd.raw_arg(line);
            }))
        };
        #[cfg(unix)]
        let (program, configure): (&str, Box<dyn FnOnce(&mut tokio::process::Command) + Send>) = {
            let development = development.clone();
            ("pnpm", Box::new(move |cmd: &mut tokio::process::Command| {
                cmd.args(["exec", "tauri", "dev", "--features", "development", "--"]);
                for entry in &development {
                    cmd.arg("--development-plugin").arg(entry);
                }
                // The tree is killed as a process GROUP; group id ==
                // the root pid because the root starts its own group.
                cmd.process_group(0);
            }))
        };
        let _ = crate::spawn::spawn_leashed_until_ready_tree(
            global,
            "viewer",
            std::path::Path::new(program),
            |cmd| {
                configure(cmd);
                cmd.current_dir(&dir);
                env(cmd);
            },
        )
        .await?;
        return Ok("ready (development)".to_string());
    }

    let _ = crate::spawn::spawn_leashed_until_ready(global, "viewer", &exe, |cmd| {
        // The viewer is a WINDOWED child (the release binary is
        // GUI-subsystem, so CREATE_NO_WINDOW never hides its window,
        // only a console-subsystem dev build's console). It is leashed
        // like every other resident child: the viewer dies with the
        // daemon BY DESIGN now.
        for entry in &development {
            cmd.arg("--development-plugin").arg(entry);
        }
        env(cmd);
    })
    .await?;
    Ok("ready".to_string())
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    Ok(Response {
        listening: spawn(global, scoped).await?,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::viewer::spawn as sdk;
    use objectiveai_sdk::cli::command::viewer::spawn::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
