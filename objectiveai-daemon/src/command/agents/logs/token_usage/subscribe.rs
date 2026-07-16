//! `agents logs token-usage subscribe` — wait for an agent's stored
//! `total_tokens` snapshot to change, OR for its instance lock to
//! drop. One-shot: emits exactly one item then ends.
//!
//! Flow:
//! 1. Read the current stored value (`baseline`).
//! 2. Fast path: if `--previous` is set AND a value is stored AND it
//!    differs from `previous`, return that value immediately.
//! 3. Otherwise subscribe: race the token-usage change (postgres
//!    LISTEN, filtered + value-compared against `baseline`) against the
//!    instance-lock release. On change → `Item`; on lock drop → a final
//!    change check, else `"agents_inactive"`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::logs::token_usage::subscribe::{
    AgentsInactiveTag, Request, ResponseItem, TokenUsage,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once(item: Result<ResponseItem, Error>) -> ItemStream {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let db = global.db_client().await?.clone();
    let state_dir = scoped.filesystem.state_dir();
    let aih = request.agent_instance_hierarchy;
    let previous = request.previous;

    let baseline = crate::db::logs::get_agent_token_usage(&db, &aih).await?;

    // Fast path: `--previous` given and the stored value already
    // differs — return it without subscribing.
    if let (Some(prev), Some(current)) = (previous, baseline) {
        if current != prev {
            return Ok(once(Ok(ResponseItem::Item(TokenUsage {
                agent_instance_hierarchy: aih,
                total_tokens: current,
            }))));
        }
    }

    // Subscribe: race token-usage change vs instance-lock release. The
    // command is one-shot, so we await the race here and emit a single
    // item (rather than a generator that yields once).
    let (lock_dir, lock_key) =
        crate::command::agents::locks::agent_instance_lock(&state_dir, &aih);

    let item: Result<ResponseItem, Error> = tokio::select! {
        result = crate::db::logs::wait_for_token_usage_change(&db, &aih, baseline) => {
            result
                .map(|total_tokens| ResponseItem::Item(TokenUsage {
                    agent_instance_hierarchy: aih.clone(),
                    total_tokens,
                }))
                .map_err(Error::from)
        }
        () = crate::command::agents::locks::wait_released(global.agent_locks(), &lock_dir, &lock_key) => {
            // A change may have landed as the lock dropped — report it
            // rather than a bare inactive signal.
            match crate::db::logs::get_agent_token_usage(&db, &aih).await {
                Ok(current) if current != baseline => match current {
                    Some(total_tokens) => Ok(ResponseItem::Item(TokenUsage {
                        agent_instance_hierarchy: aih.clone(),
                        total_tokens,
                    })),
                    None => Ok(ResponseItem::AgentsInactive(
                        AgentsInactiveTag::AgentsInactive,
                    )),
                },
                Ok(_) => Ok(ResponseItem::AgentsInactive(
                    AgentsInactiveTag::AgentsInactive,
                )),
                Err(e) => Err(Error::from(e)),
            }
        }
    };

    Ok(once(item))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::logs::token_usage::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::logs::token_usage::subscribe::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::logs::token_usage::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::logs::token_usage::subscribe::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
