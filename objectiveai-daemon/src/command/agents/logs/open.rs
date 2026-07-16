//! `agents logs read id <id>` — resolve a `objectiveai.messages."index"`
//! to its typed [`Response`] variant. The dispatch logic lives in
//! [`crate::db::logs::read_by_id`]; this handler is a thin wrapper.

use objectiveai_sdk::cli::command::agents::logs::open::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, _scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    crate::db::logs::read_by_id(global.db_client().await?, request.id)
        .await?
        .ok_or_else(|| {
            Error::Filesystem(crate::filesystem::Error::NotFound(format!(
                "objectiveai.messages row at index {}",
                request.id
            )))
        })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::logs::open as sdk;
    use objectiveai_sdk::cli::command::agents::logs::open::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::logs::open as sdk;
    use objectiveai_sdk::cli::command::agents::logs::open::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
