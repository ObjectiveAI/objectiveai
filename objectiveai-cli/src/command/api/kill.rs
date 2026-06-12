//! `api kill --global` — terminate the api server by killing the
//! owner(s) of its machine-wide lock at `<dir>/bin/locks` key
//! `api`. Idempotent: a count of zero is not an error. There is
//! exactly one api lock, so no concurrency is needed.

use objectiveai_sdk::cli::command::api::kill::{Request, Response};

use crate::command::kill_helpers::kill_lock_owners;
use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    // The SDK `TryFrom` already enforced --global.
    let locks_dir = ctx.filesystem.bin_dir().join("locks");
    let killed = kill_lock_owners(locks_dir, "api").await?;
    Ok(Response { killed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::kill as sdk;
    use objectiveai_sdk::cli::command::api::kill::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::kill as sdk;
    use objectiveai_sdk::cli::command::api::kill::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
