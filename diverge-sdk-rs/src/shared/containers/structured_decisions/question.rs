//! One question of a structured decision.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

use super::ChoiceOption;

/// One question: its id, its kind, what it asks, and what its kind
/// adds. Internally tagged by `type`, snake case: `"noul"`,
/// `"choice"`, `"score"`. Every kind has `id` and `instructions`; a
/// choice adds `options` and a score adds `levels`, and no kind has
/// any other member.
///
/// The id is the caller's, and is the key its [`Answer`](super::Answer)
/// comes back under. What makes a list of questions valid is
/// [`validate`](super::validate)'s to say.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Question {
    /// A single number: nothing beside the id and the instructions.
    /// Answered by [`Answer::Noul`](super::Answer::Noul).
    Noul {
        /// The question's id: what its answer is keyed by.
        id: String,
        /// What is asked, as content parts, in order.
        instructions: Vec<ContentBlock>,
    },
    /// One among options. Answered by
    /// [`Answer::Choice`](super::Answer::Choice), a probability for
    /// each option's value.
    Choice {
        /// The question's id: what its answer is keyed by.
        id: String,
        /// What is asked, as content parts, in order.
        instructions: Vec<ContentBlock>,
        /// The options, in order: see [`ChoiceOption`].
        options: Vec<ChoiceOption>,
    },
    /// A place along levels. Answered by
    /// [`Answer::Score`](super::Answer::Score), a probability for
    /// each level.
    Score {
        /// The question's id: what its answer is keyed by.
        id: String,
        /// What is asked, as content parts, in order.
        instructions: Vec<ContentBlock>,
        /// The levels, in order, each content parts describing it.
        levels: Vec<Vec<ContentBlock>>,
    },
}

impl Question {
    /// The question's id, whichever kind it is.
    pub fn id(&self) -> &str {
        match self {
            Question::Noul { id, .. } | Question::Choice { id, .. } | Question::Score { id, .. } => id,
        }
    }

    /// What is asked, whichever kind it is.
    pub fn instructions(&self) -> &[ContentBlock] {
        match self {
            Question::Noul { instructions, .. } | Question::Choice { instructions, .. } | Question::Score { instructions, .. } => {
                instructions
            }
        }
    }
}
