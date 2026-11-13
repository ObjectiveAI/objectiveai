//! A structured decision put to an agent.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::message::structured_decisions::client::request;
use diverge_sdk::daemon::endpoints::agents::message::structured_decisions::server::response::Frame;
use diverge_sdk::shared::containers::info::{Document, FalseOr};
use diverge_sdk::shared::containers::structured_decisions::validate;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use super::{Refused, judged, undocumented};
use crate::containers::message;
use crate::daemon::Daemon;
use crate::judge::Who;
use crate::serve::{Turn, reply};
use crate::store::agents;

/// Deliver the decision and finish the scope: the answer, then the
/// finish. `turn` is the request's place in the order its connection
/// sent it, which the decision is admitted in.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, turn: Turn, who: Who, daemon: &Arc<Daemon>) {
    let answer = serve(&scope, frame, turn, who, daemon).await;
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `message` grant at all, or for an agent the
/// grants do not reach; the error for an agent that is not there; the
/// refusal [`held`] makes of the decision; else the decision admitted
/// to the agent's outbox — the one chat messages are admitted to, in
/// the one order — the scope open until its fate is known:
/// `Delivered` once the agent's program has it, which is when the run
/// it starts has begun, after everything admitted before it has
/// finished; or the error. The admission follows the rules a chat
/// message's does: the lookups at once, then the [`Turn`], then only
/// the fate waited for.
async fn serve(scope: &ScopeHandle, frame: request::Frame, mut turn: Turn, who: Who, daemon: &Arc<Daemon>) -> Frame {
    let (agent, sender, document) = match judged(scope, &frame.agent, who, daemon).await {
        Ok(judged) => judged,
        Err(Refused::Forbidden) => return Frame::Forbidden,
        Err(Refused::Error(error)) => return Frame::Error(error),
    };
    if let Err(answer) = held(&agent, document.as_ref(), &frame) {
        return answer;
    }
    turn.come().await;
    let what = message::What::Decision { content: frame.content, questions: frame.questions };
    let fate = message::admit(daemon, agent, what, sender).await;
    drop(turn);
    match fate.await {
        Ok(message::Fate::Delivered) => Frame::Delivered,
        Ok(message::Fate::Error(error)) => Frame::Error(error),
        Err(_) => Frame::Error(reply::failure(&"the decision's delivery ended before its fate was known")),
    }
}

/// The decision held to what the agent's image's document says and to
/// what a decision's questions must be, before it is admitted: `Ok`
/// when it may go on, else the answer. Nothing is asked of a provider
/// or a container here, nothing is started, and nothing is written to
/// the agent's log: a decision refused here reached nothing. The
/// rules, in order:
///
/// 1. `document` is the one on record for the image of the agent's
///    template, read with the agent and never fetched. With no
///    template, or no document on record, the answer is the `image`
///    error.
/// 2. A document whose `structured_decisions` is `false` is
///    `NoStructuredDecisions`: the image takes no decision.
/// 3. The questions are valid, as
///    [`validate`] states; else the `questions` error, naming the
///    question and saying why.
/// 4. The content, the instructions, the options' descriptions and
///    the levels are held to the document's modalities, as
///    [`StructuredDecisions::lacking`](diverge_sdk::shared::containers::info::StructuredDecisions::lacking)
///    states; anything lacking is `UnsupportedModalities`, saying what
///    and where.
fn held(agent: &agents::Agent, document: Option<&Document>, frame: &request::Frame) -> Result<(), Frame> {
    // Rule 1.
    let Some(document) = document else {
        return Err(Frame::Error(undocumented(agent)));
    };
    // Rule 2.
    let FalseOr::Is(decisions) = &document.structured_decisions else {
        return Err(Frame::NoStructuredDecisions);
    };
    // Rule 3.
    if let Err(invalid) = validate(&frame.questions) {
        return Err(Frame::Error(reply::questions(&invalid)));
    }
    // Rule 4.
    let lacking = decisions.lacking(&frame.content, &frame.questions);
    if lacking.is_empty() { Ok(()) } else { Err(Frame::UnsupportedModalities(lacking)) }
}
