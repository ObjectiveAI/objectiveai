//! A chat message, held to what the agent's image says.

use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::shared::containers::info::{Document, FalseOr, Lacking};
use rmcp::model::ContentBlock;

use super::undocumented;
use crate::store::agents;

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
///    each as the prompt's, with no choice's.
pub fn held(agent: &agents::Agent, document: Option<&Document>, content: &[ContentBlock]) -> Result<(), Frame> {
    let Some(document) = document else {
        return Err(Frame::Error(undocumented(agent)));
    };
    let FalseOr::Is(chat) = &document.chat else {
        return Err(Frame::NoChat);
    };
    let prompt = chat.prompt_modalities.lacking(content);
    if prompt.is_empty() { Ok(()) } else { Err(Frame::UnsupportedModalities(Lacking { prompt, instruction: Vec::new(), choice: Vec::new() })) }
}
