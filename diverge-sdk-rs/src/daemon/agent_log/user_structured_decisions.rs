//! A structured decision a caller sent, and who sent it.

use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::provider::endpoints::containers::agents::run::server::response::UserStructuredDecisionsChunk;

/// A structured decision as it landed: the chunk, the id the log gave
/// it, and who sent it.
///
/// The id is the log's, a whole number, and is how a
/// [`StructuredDecisionsResponse`](super::StructuredDecisionsResponse)
/// is read against the decision it answers. The rules, for one
/// agent's log:
///
/// 1. A decision takes one more than the highest id any decision or
///    response before it in the log has, and `0` where none has one.
/// 2. The first response after a decision that no response has yet
///    answered takes that decision's id.
/// 3. Any other response — a second after one decision, or one with
///    no decision before it — takes one more than the highest id
///    before it, and `0` where none has one.
///
/// So ids never go down, a decision and its response share one, and
/// `decision, response, response, decision, response` is `0, 0, 1, 2,
/// 2`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct UserStructuredDecisions {
    /// The log's id for the decision: see the rules above.
    pub id: u64,
    /// Who sent it, where the daemon knows: the sender of the message
    /// the chunk's key names. Absent for a key the daemon did not
    /// send.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<Creator>,
    /// The decision, as the agent's program said it landed.
    #[serde(flatten)]
    pub chunk: UserStructuredDecisionsChunk,
}
