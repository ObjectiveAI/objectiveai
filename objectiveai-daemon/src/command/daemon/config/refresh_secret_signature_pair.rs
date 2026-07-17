//! `daemon config refresh-secret-signature-pair` — generate a fresh
//! random `(secret, signature)` pair, persist it as `daemon.secret` /
//! `daemon.signature` in the on-disk config at the requested scope,
//! and return it. The pair generation is the same one-way scheme the
//! viewer auth uses (`sha256=<hex(SHA256(secret))>` — knowing the
//! signature does not reveal the secret). Nothing consumes the stored
//! pair yet; the daemon still reads its bare `SECRET`/`SIGNATURE` env.

use objectiveai_sdk::cli::command::daemon::config::refresh_secret_signature_pair::{
    Request, Response,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    _global: &GlobalContext,
    scoped: &ScopedContext,
    _request: Request,
) -> Result<Response, Error> {
    let pair = crate::filesystem::config::generate_viewer_secret_signature_pair();
    let mut config = scoped.filesystem.read_config().await?;
    config.daemon().set_secret(pair.secret.clone());
    config.daemon().set_signature(pair.signature.clone());
    scoped.filesystem.write_config(&config).await?;
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
