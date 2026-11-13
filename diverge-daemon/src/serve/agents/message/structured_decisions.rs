//! A structured decision, held to what the agent's image says and to
//! what its questions must be.

use diverge_sdk::daemon::endpoints::agents::message::server::response::Frame;
use diverge_sdk::shared::containers::info::{Document, FalseOr};
use diverge_sdk::shared::containers::structured_decisions::{Question, validate};
use rmcp::model::ContentBlock;

use super::undocumented;
use crate::serve::reply;
use crate::store::agents;

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
pub fn held(agent: &agents::Agent, document: Option<&Document>, content: &[ContentBlock], questions: &[Question]) -> Result<(), Frame> {
    // Rule 1.
    let Some(document) = document else {
        return Err(Frame::Error(undocumented(agent)));
    };
    // Rule 2.
    let FalseOr::Is(decisions) = &document.structured_decisions else {
        return Err(Frame::NoStructuredDecisions);
    };
    // Rule 3.
    if let Err(invalid) = validate(questions) {
        return Err(Frame::Error(reply::questions(&invalid)));
    }
    // Rule 4.
    let lacking = decisions.lacking(content, questions);
    if lacking.is_empty() { Ok(()) } else { Err(Frame::UnsupportedModalities(lacking)) }
}
