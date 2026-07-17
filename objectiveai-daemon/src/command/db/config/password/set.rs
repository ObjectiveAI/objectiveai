//! `config db password set` — write `db.password` to on-disk config.

use objectiveai_sdk::cli::command::db::config::password::set::{Request, Response};

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
    config.db().set_password(request.value);
    scoped.filesystem.write_config(&config).await?;
    // Sweep again after the write (best-effort): a concurrent rebuild
    // may have respawned the db against the OLD config mid-set.
    crate::command::kill_helpers::kill_db_after_config_change(global).await;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::db::config::password::set as sdk;
    use objectiveai_sdk::cli::command::db::config::password::set::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::db::config::password::set as sdk;
    use objectiveai_sdk::cli::command::db::config::password::set::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
