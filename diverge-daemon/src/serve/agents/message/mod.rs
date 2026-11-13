//! Asking an agent something: a chat message, or a structured
//! decision.
//!
//! Its two requests share who may send and how the agent is found —
//! [`judged`] — and part ways at what the agent's image's document is
//! held to. Both admit to the agent's one outbox, in the order their
//! connection sent them.

use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::grant::agents::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::state;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store::{self, agents, agents_templates, image_info};

pub mod chat;
pub mod structured_decisions;

/// Why the agent was not found for the sender: the answer either
/// request gives, in its own frame.
pub enum Refused {
    /// No `message` grant at all, or none reaching the agent.
    Forbidden,
    /// The agent is not there, or the store failed.
    Error(Error),
}

/// The agent `reference` names, who sends to it, and the document on
/// record for its template's image — `None` when the template is gone
/// or its image has none on record — once the grants let the request
/// through, the scope then told so; else why not. The grant is
/// `message` over agents, for a chat message and a structured decision
/// alike.
pub async fn judged(
    scope: &ScopeHandle,
    reference: &reference::Agent,
    who: Who,
    daemon: &Arc<Daemon>,
) -> Result<(agents::Agent, Creator, Option<Document>), Refused> {
    let failed = |error: store::Error| Refused::Error(reply::failure(&error));
    let mut conn = daemon.store.acquire().await.map_err(failed)?;
    let Some(standing) = Standing::of(&mut conn, who).await.map_err(failed)? else {
        return Err(Refused::Forbidden);
    };
    if !judge::agents::holds(&standing, Over::Message) {
        return Err(Refused::Forbidden);
    }
    let Some(agent) = agents::by_reference(&mut conn, reference, false).await.map_err(failed)? else {
        return Err(Refused::Error(reply::failure(&"the agent named is none the daemon has")));
    };
    let document = match agents_templates::by_id(&mut conn, &agent.template, false).await.map_err(failed)? {
        Some(record) => image_info::by_digest(&mut conn, &record.template.image.digest).await.map_err(failed)?,
        None => None,
    };
    drop(conn);
    if !judge::agents::over(&standing, Over::Message, &agent, state(daemon, agent.id).await) {
        return Err(Refused::Forbidden);
    }
    scope.allowed().await;
    Ok((agent, standing.creator.clone(), document))
}

/// The `image` error for an agent whose template is gone, or whose
/// image has no document on record: what either request answers
/// before it reads the document.
pub fn undocumented(agent: &agents::Agent) -> Error {
    reply::image(&format!("the agent's template `{}` is gone, or its image has no document on record", agent.template))
}
