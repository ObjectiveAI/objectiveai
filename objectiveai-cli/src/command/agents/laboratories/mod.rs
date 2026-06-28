//! `agents laboratories` — CLI-side dispatch for attach/detach/list of
//! laboratory ids on an agent target (a tag or an instance hierarchy).

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::laboratories::{Request, ResponseItem};
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::lockfile::LockClaim;

use crate::command::agents::locks;
use crate::context::Context;
use crate::db::laboratory_attachments::Target;
use crate::error::Error;

pub mod attach;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Attach(req) => {
            let value = attach::execute(ctx, req).await?;
            once(Ok(ResponseItem::Attach(value)))
        }
        Request::AttachRequestSchema(req) => {
            let value = attach::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AttachRequestSchema(value)))
        }
        Request::AttachResponseSchema(req) => {
            let value = attach::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AttachResponseSchema(value)))
        }
    };
    Ok(stream)
}

/// Resolve the agent target to its DB key AND acquire the lock(s) that
/// guard mutation for it. Shared by `attach` + `detach`.
///
/// - **Instance** (PAIH + `--agent-instance`) → the AIH lock; keyed on
///   the AIH.
/// - **Tag, GROUPED** → the tag lock; keyed on the tag.
/// - **Tag, BOUND** → the tag lock AND the bound AIH lock (tag order
///   first, for deadlock-freedom); keyed on the **tag**.
/// - **Ref** (a direct agent spec) → error (no tag/AIH to key on).
///
/// Locks are taken with `try_acquire`, so a live owner ⇒ an "active"
/// error. On any error the partial claims are released here. On success
/// the caller MUST [`release_all`] after its DB op — `LockClaim` does
/// not release on drop.
pub(super) async fn lock_target(
    ctx: &Context,
    selector: &AgentSelector,
) -> Result<(Target, Vec<LockClaim>), Error> {
    let state_dir = ctx.filesystem.state_dir();
    match selector {
        AgentSelector::Ref { .. } => Err(Error::LaboratoryRefTarget),
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            let aih = format!("{parent}/{agent_instance}");
            let (dir, key) = locks::agent_instance_lock(&state_dir, &aih);
            let claim = objectiveai_sdk::lockfile::try_acquire(&dir, &key, "")
                .await
                .ok_or(Error::AgentInstanceActive {
                    agent_instance_hierarchy: aih.clone(),
                })?;
            Ok((Target::Aih(aih), vec![claim]))
        }
        AgentSelector::Tag { agent_tag } => {
            let (tdir, tkey) = locks::agent_tag_lock(&state_dir, agent_tag);
            let tag_claim = objectiveai_sdk::lockfile::try_acquire(&tdir, &tkey, "")
                .await
                .ok_or_else(|| Error::AgentTagActive {
                    tag: agent_tag.clone(),
                })?;
            let pool = ctx.db_client().await?;
            match crate::db::tags::lookup(pool, agent_tag).await {
                Ok(crate::db::tags::LookupState::Absent) => {
                    let _ = tag_claim.release();
                    Err(Error::TagNotFound(agent_tag.clone()))
                }
                Ok(crate::db::tags::LookupState::Grouped { .. }) => {
                    Ok((Target::Tag(agent_tag.clone()), vec![tag_claim]))
                }
                Ok(crate::db::tags::LookupState::Bound {
                    agent_instance_hierarchy,
                }) => {
                    let (idir, ikey) =
                        locks::agent_instance_lock(&state_dir, &agent_instance_hierarchy);
                    match objectiveai_sdk::lockfile::try_acquire(&idir, &ikey, "").await {
                        Some(aih_claim) => Ok((
                            Target::Tag(agent_tag.clone()),
                            vec![tag_claim, aih_claim],
                        )),
                        None => {
                            let _ = tag_claim.release();
                            Err(Error::AgentInstanceActive {
                                agent_instance_hierarchy,
                            })
                        }
                    }
                }
                Err(e) => {
                    let _ = tag_claim.release();
                    Err(e.into())
                }
            }
        }
    }
}

/// Release every claim (best-effort). Call after the DB op on both the
/// success and error paths — claims do not release on drop.
pub(super) fn release_all(claims: Vec<LockClaim>) {
    for claim in claims {
        let _ = claim.release();
    }
}
