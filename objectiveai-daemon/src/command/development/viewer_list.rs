//! `development plugins viewer list` — every viewer development registration.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::development::plugins::viewer::list::{
    Request, ResponseItem,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    _request: Request,
) -> Result<ItemStream, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development plugins viewer list requires the resident daemon".to_string(),
        )
    })?;

    // Snapshot, not a live view: the registry is a `DashMap` and
    // holding iteration across the stream would pin its shards for as
    // long as the caller takes to read. There are never many of these.
    let registrations = hubs.development_plugins.viewer.list();

    Ok(Box::pin(futures::stream::iter(registrations.into_iter().map(
        |((owner, name, version), path)| {
            Ok(ResponseItem {
                owner,
                name,
                version,
                path: path.to_string_lossy().into_owned(),
            })
        },
    ))))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::plugins::viewer::list as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::list::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::development::plugins::viewer::list as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::list::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
