//! One question's answer, as the log keeps it.

use indexmap::IndexMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::shared::containers::structured_decisions;

/// The answer to one question as the agent's log keeps it: what the
/// agent's program said — see
/// [`structured_decisions::Answer`] — and what the daemon made of it.
/// Untagged, each shape taking no member but its own.
///
/// | question | answer |
/// |---|---|
/// | noul | `{"noul": 0.25}` |
/// | choice | `{"choice": "<option value>", "probabilities": {"<option value>": 0.7, …}}` |
/// | score | `{"score": 1.7, "probabilities": [0.1, 0.2, …]}` |
///
/// The `choice` and the `score` are never the program's: its answer
/// has neither, and the daemon computes both as it writes the item,
/// by the rules [`Answer::of`] states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
pub enum Answer {
    /// A score's: see [`Score`].
    Score(Score),
    /// A choice's: see [`Choice`].
    Choice(Choice),
    /// A noul's: as the program said it.
    Noul(structured_decisions::Noul),
}

/// A choice's answer, kept: the winning option, and the probabilities
/// it won by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    /// The option that won, by its value: rule 1 of [`Answer::of`].
    pub choice: String,
    /// Each option's probability, keyed by its value, in the order the
    /// agent's program stated them.
    pub probabilities: IndexMap<String, Decimal>,
}

/// A score's answer, kept: the score, and the probabilities it is the
/// sum of.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Score {
    /// The score: rule 2 of [`Answer::of`].
    pub score: Decimal,
    /// Each level's probability, in the levels' order.
    pub probabilities: Vec<Decimal>,
}

impl Answer {
    /// The answer the log keeps for `said`, an answer of the agent's
    /// program. The rules:
    ///
    /// 1. **A choice's `choice` is the key with the highest
    ///    probability.** Where several keys share the highest, the one
    ///    that sorts first as text, byte by byte. With no
    ///    probabilities at all it is the empty string.
    /// 2. **A score's `score` is the sum, over its probabilities, of
    ///    each probability times its position**, the first counting
    ///    `0`, the second `1`, and so on. With no probabilities it is
    ///    `0`.
    /// 3. **Nothing is checked.** The answer is not read against its
    ///    question, which only a schema held it to: probabilities that
    ///    do not sum to one, a key that is no option's value, more or
    ///    fewer probabilities than the question has levels, and a
    ///    score past the last level are each computed as given.
    /// 4. **Nothing fails.** A product or a sum too large for a
    ///    decimal is the largest, or the smallest, a decimal holds:
    ///    the item is kept whatever the numbers are.
    /// 5. **A noul is kept as said.**
    pub fn of(said: structured_decisions::Answer) -> Self {
        match said {
            // Rule 1.
            structured_decisions::Answer::Choice(said) => {
                let mut won: Option<(&String, &Decimal)> = None;
                for (value, probability) in &said.probabilities {
                    let wins = match won {
                        None => true,
                        Some((held, highest)) => probability > highest || (probability == highest && value < held),
                    };
                    if wins {
                        won = Some((value, probability));
                    }
                }
                Answer::Choice(Choice {
                    choice: won.map(|(value, _)| value.clone()).unwrap_or_default(),
                    probabilities: said.probabilities,
                })
            }
            // Rules 2 and 4.
            structured_decisions::Answer::Score(said) => {
                let score = said
                    .probabilities
                    .iter()
                    .enumerate()
                    .fold(Decimal::ZERO, |sum, (level, probability)| sum.saturating_add(probability.saturating_mul(Decimal::from(level))));
                Answer::Score(Score { score, probabilities: said.probabilities })
            }
            // Rule 5.
            structured_decisions::Answer::Noul(said) => Answer::Noul(said),
        }
    }
}
