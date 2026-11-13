//! A chat message to an agent.

use diverge_sdk::daemon::endpoints::agents::message::chat::client::request;
use diverge_sdk::daemon::endpoints::agents::message::chat::server::response::Frame;
use diverge_sdk::shared::containers::info::{Document, FalseOr};
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use std::sync::Arc;

use super::{Refused, judged, undocumented};
use crate::containers::message;
use crate::daemon::Daemon;
use crate::judge::Who;
use crate::serve::{Turn, reply};
use crate::store::agents;

/// Deliver the message and finish the scope: the answer, then the
/// finish. `turn` is the request's place in the order its connection
/// sent it, which the message is admitted in.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, turn: Turn, who: Who, daemon: &Arc<Daemon>) {
    let answer = serve(&scope, frame, turn, who, daemon).await;
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` with no `message` grant at all, or for an agent the
/// grants do not reach; the error for an agent that is not there; the
/// refusal the agent's image's document makes of the message, see
/// [`held`]; else
/// the message admitted to the agent's outbox, the scope open until
/// its fate is known: `Delivered` once the agent's program has it —
/// the loop it started, or the program's own queue — or the error: a
/// container that could not be started or activated, or the
/// container's own. Nothing takes a message back: the client opens no
/// channel on the scope, and the answer is the fate. The rules of the
/// admission, which make the messages one connection sends reach the
/// agent in the order it sent them:
///
/// 1. **The lookups first, and at once.** Who sends, the agent named
///    and what the grants make of it are read as the handler starts,
///    each message's beside every other's.
/// 2. **Then the turn.** The message waits for its [`Turn`] to come —
///    every request its connection sent before it let go, which for a
///    message is admitted — and is admitted to the outbox:
///    [`message::admit`]. The wait is only for the lookups of the
///    messages before it.
/// 3. **Then the turn is let go**, and only the fate is waited for. A
///    message refused lets its turn go by returning, as rule 4 of
///    [`Turn`] states.
/// 4. **A fate never said** — the agent's pump died with the message
///    waiting — is the daemon's error.
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
    let fate = message::admit(daemon, agent, message::What::Chat(frame.content), sender).await;
    drop(turn);
    match fate.await {
        Ok(message::Fate::Delivered) => Frame::Delivered,
        Ok(message::Fate::Error(error)) => Frame::Error(error),
        Err(_) => Frame::Error(reply::failure(&"the message's delivery ended before its fate was known")),
    }
}

/// The message held to what the agent's image's document says, before
/// it is admitted: `Ok` when it may go on, else the answer. Nothing is
/// asked of a provider or a container here, nothing is started, and
/// nothing is written to the agent's log: a message refused here
/// reached nothing. The rules, in order:
///
/// 1. `document` is the one on record for the image of the agent's
///    template, read with the agent and never fetched: a template's
///    creation leaves it there. With no template, or no document on
///    record, the answer is the `image` error.
/// 2. A document whose `chat` is `false` is `NoChat`: the image takes
///    no message, whatever it carries.
/// 3. Else the content is held to the `chat`'s `prompt_modalities`, as
///    [`Modalities::lacking`](diverge_sdk::shared::containers::info::Modalities::lacking)
///    states; any modality lacking is `UnsupportedModalities`, naming
///    each.
fn held(agent: &agents::Agent, document: Option<&Document>, frame: &request::Frame) -> Result<(), Frame> {
    let Some(document) = document else {
        return Err(Frame::Error(undocumented(agent)));
    };
    let FalseOr::Is(chat) = &document.chat else {
        return Err(Frame::NoChat);
    };
    let lacking = chat.prompt_modalities.lacking(&frame.content);
    if lacking.is_empty() { Ok(()) } else { Err(Frame::UnsupportedModalities(lacking)) }
}
