//! `daemon config set` — FULL-REPLACE the whole `daemon` config
//! section from one wire object. The section's values are LINKED (a
//! secret and its derived signature, the address they authorize), so
//! per-field wire setters were retired: every mutation states one
//! complete consistent object, and omitted fields are cleared.
//!
//! A viewer RUNNING at set time is respawned AFTER the write — the
//! viewer's whole daemon-facing config (`DAEMON_ADDRESS` /
//! `DAEMON_SIGNATURE`) is frozen into its env at spawn, so a config
//! change can only reach it through a fresh spawn. A viewer that
//! isn't running is left alone (the next `viewer spawn` picks the
//! values up itself). The daemon's own bind still comes from bare
//! env until the config wiring lands — the respawned viewer gets the
//! daemon's LIVE address and client signature, same as any spawn.

use objectiveai_sdk::cli::command::daemon::config::set::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    // Sampled BEFORE the write: only a viewer the user already has up
    // gets bounced; the set never turns into a surprise viewer launch.
    let viewer_was_running = global.server_child_alive("viewer");
    let mut config = scoped.filesystem.read_config().await?;
    config.daemon = Some(crate::filesystem::config::DaemonConfig {
        address: request.value.address,
        secret: request.value.secret,
        signature: request.value.signature,
    });
    scoped.filesystem.write_config(&config).await?;
    crate::command::kill_helpers::respawn_viewer_after_config_change(
        global,
        scoped,
        viewer_was_running,
    )
    .await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::daemon::config::set as sdk;
    use objectiveai_sdk::cli::command::daemon::config::set::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::config::set as sdk;
    use objectiveai_sdk::cli::command::daemon::config::set::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
