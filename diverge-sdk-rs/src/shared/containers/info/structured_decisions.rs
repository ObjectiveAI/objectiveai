//! What an agent that makes structured decisions says of itself.

use serde::{Deserialize, Serialize};

use rmcp::model::ContentBlock;

use super::{Modalities, Modality};
use crate::shared::containers::structured_decisions::Question;

/// How an agent's program makes structured decisions: three members,
/// all required, and no other.
///
/// Stated by an agent's image, in the frontmatter's
/// `structured_decisions`. The daemon holds a decision to it: one for
/// an agent whose image states `false` is refused, and so is one
/// carrying what these do not take, see [`lacking`](Self::lacking).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StructuredDecisions {
    /// What the prompt of a decision — its `content` — may carry
    /// beside text.
    pub prompt_modalities: Modalities,
    /// What the instructions of a decision's questions may carry
    /// beside text.
    pub instruction_modalities: Modalities,
    /// What a choice of a decision may carry beside text.
    pub choice_modalities: Modalities,
}

/// What a decision carries that an image does not take, by where it
/// carries it: see [`StructuredDecisions::lacking`]. Every list as
/// [`Modalities::lacking`] orders them. A chat message has a prompt
/// and nothing else, so its `instruction` and `choice` are empty.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Lacking {
    /// Carried by the prompt and not taken there.
    pub prompt: Vec<Modality>,
    /// Carried by the questions' instructions and not taken there.
    pub instruction: Vec<Modality>,
    /// Carried by the choices and not taken there.
    pub choice: Vec<Modality>,
}

impl Lacking {
    /// Whether nothing is lacking: the decision may go on.
    pub fn is_empty(&self) -> bool {
        self.prompt.is_empty() && self.instruction.is_empty() && self.choice.is_empty()
    }
}

impl StructuredDecisions {
    /// The modalities a decision carries that these do not take. The
    /// rules:
    ///
    /// 1. The prompt is the decision's `content`, held to
    ///    [`prompt_modalities`](Self::prompt_modalities).
    /// 2. The instructions are every question's `instructions`, held
    ///    to [`instruction_modalities`](Self::instruction_modalities).
    /// 3. The choices are every option's `description` of every
    ///    choice, and every level of every score, held to
    ///    [`choice_modalities`](Self::choice_modalities). An option's
    ///    `value` is text and carries nothing.
    /// 4. Each block is held as [`Modalities::lacking`] holds one.
    pub fn lacking(&self, content: &[ContentBlock], questions: &[Question]) -> Lacking {
        let instruction = questions.iter().flat_map(|question| question.instructions().iter());
        let choice = questions.iter().flat_map(|question| -> Box<dyn Iterator<Item = &ContentBlock> + '_> {
            match question {
                Question::Noul { .. } => Box::new(std::iter::empty()),
                Question::Choice { options, .. } => Box::new(options.iter().filter_map(|option| option.description.as_deref()).flatten()),
                Question::Score { levels, .. } => Box::new(levels.iter().flatten()),
            }
        });
        Lacking {
            prompt: self.prompt_modalities.lacking(content),
            instruction: self.instruction_modalities.lacking_among(instruction),
            choice: self.choice_modalities.lacking_among(choice),
        }
    }
}
