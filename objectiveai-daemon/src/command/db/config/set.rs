//! `db config set` — FULL-REPLACE the whole `db` config section from
//! one wire object. The section's values are LINKED (an address, the
//! user/password that authenticate there, the database they open), so
//! per-field wire setters were retired: every mutation states one
//! complete consistent object, and omitted fields are cleared. The
//! write keeps the per-field setters' kill bracket — the running db
//! was spawned under the config this replaces.

use objectiveai_sdk::cli::command::db::config::set::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    // Retire the running db BEFORE the write: it was spawned with (and
    // its cached handle resolved under) the config this set replaces,
    // and a db we cannot kill must not survive the change — so a kill
    // failure aborts the set. The cached DbHandle is invalidated in
    // the same gate-held section.
    crate::command::kill_helpers::kill_db_before_config_change(global).await?;
    let mut config = scoped.filesystem.read_config().await?;
    config.db = Some(crate::filesystem::config::DbConfig {
        address: request.value.address,
        user: request.value.user,
        password: request.value.password,
        database: request.value.database,
    });
    scoped.filesystem.write_config(&config).await?;
    // Sweep again after the write (best-effort): a concurrent rebuild
    // may have respawned the db against the OLD config mid-set.
    crate::command::kill_helpers::kill_db_after_config_change(global).await;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::db::config::set as sdk;
    use objectiveai_sdk::cli::command::db::config::set::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::db::config::set as sdk;
    use objectiveai_sdk::cli::command::db::config::set::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
