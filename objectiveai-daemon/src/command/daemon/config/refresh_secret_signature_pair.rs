//! `daemon config refresh-secret-signature-pair` — generate a fresh
//! random `(secret, signature)` pair, FULL-REPLACE the `daemon`
//! config section with it (`address` is cleared: the section's values
//! are linked, and a rotated pair makes no claim about the old
//! address), and return the pair. The generation is the same one-way
//! scheme the viewer auth uses (`sha256=<hex(SHA256(secret))>` —
//! knowing the signature does not reveal the secret). Nothing
//! consumes the stored pair yet; the daemon still reads its bare
//! `SECRET`/`SIGNATURE` env.
//!
//! Like `daemon config set`, a viewer RUNNING at refresh time is
//! respawned after the write (see
//! [`crate::command::kill_helpers::respawn_viewer_after_config_change`]).

use objectiveai_sdk::cli::command::daemon::config::refresh_secret_signature_pair::{
    Request, Response,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    _request: Request,
) -> Result<Response, Error> {
    let viewer_was_running = global.server_child_alive("viewer");
    let pair = crate::filesystem::config::generate_viewer_secret_signature_pair();
    let mut config = scoped.filesystem.read_config().await?;
    config.daemon = Some(crate::filesystem::config::DaemonConfig {
        address: None,
        secret: Some(pair.secret.clone()),
        signature: Some(pair.signature.clone()),
    });
    scoped.filesystem.write_config(&config).await?;
    crate::command::kill_helpers::respawn_viewer_after_config_change(
        global,
        scoped,
        viewer_was_running,
    )
    .await?;
    Ok(Response {
        secret: pair.secret,
        signature: pair.signature,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::daemon::config::refresh_secret_signature_pair as sdk;
    use objectiveai_sdk::cli::command::daemon::config::refresh_secret_signature_pair::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::daemon::config::refresh_secret_signature_pair as sdk;
    use objectiveai_sdk::cli::command::daemon::config::refresh_secret_signature_pair::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
