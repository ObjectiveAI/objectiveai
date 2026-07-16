//! `plugins logs list` — stream captured plugin stderr lines for one
//! plugin coordinate (owner/name/version), ascending by the BIGSERIAL
//! `"index"` cursor (`--after-id` / `--limit` paginate).

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::plugins::logs::list::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(global: &GlobalContext, _scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let db = global.db_client().await?.clone();
    let items = crate::db::logs::read_plugin_messages(
        &db,
        &request.owner,
        &request.name,
        &request.version,
        request.after_id,
        request.limit,
    )
    .await
    .map_err(Error::from)?;
    let stream = async_stream::stream! {
        for item in items {
            yield Ok(item);
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::logs::list as sdk;
    use objectiveai_sdk::cli::command::plugins::logs::list::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::logs::list as sdk;
    use objectiveai_sdk::cli::command::plugins::logs::list::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
