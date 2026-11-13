//! What a decision's questions are held to.

use std::collections::HashSet;
use std::fmt;

use rmcp::model::ContentBlock;

use super::Question;

/// Why a list of questions is not valid: the question it is about, by
/// its id — empty where it is about the list itself — and the reason,
/// in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    /// The id of the question at fault, or empty.
    pub question: String,
    /// What is wrong with it.
    pub reason: &'static str,
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.question.is_empty() {
            true => f.write_str(self.reason),
            false => write!(f, "question `{}`: {}", self.question, self.reason),
        }
    }
}

impl std::error::Error for Invalid {}

/// Whether `content` says anything: at least one part that is not a
/// text part with no text. The rules:
///
/// 1. A text part whose text is the empty string does not count.
/// 2. A text part with any text counts, whitespace alone included:
///    nothing is trimmed.
/// 3. Every part that is not text counts — an image, audio, a
///    resource, a link — whatever it holds.
pub fn real(content: &[ContentBlock]) -> bool {
    content.iter().any(|block| match block {
        ContentBlock::Text(text) => !text.text.is_empty(),
        _ => true,
    })
}

/// Hold `questions` to what a decision's questions must be, answering
/// the first fault found, in the questions' order. The rules:
///
/// 1. There is at least one question.
/// 2. No question's id is empty, and no two questions share an id.
/// 3. Every question's instructions are [`real`].
/// 4. A choice has at least two options. No option's value is empty,
///    and no two options of one question share a value: the values key
///    the answer. An option's description is absent, or [`real`].
/// 5. A score has at least two levels, and every level is [`real`].
pub fn validate(questions: &[Question]) -> Result<(), Invalid> {
    let invalid = |question: &Question, reason| Invalid { question: question.id().to_string(), reason };
    // Rule 1.
    if questions.is_empty() {
        return Err(Invalid { question: String::new(), reason: "a decision has at least one question" });
    }
    let mut ids = HashSet::new();
    for question in questions {
        // Rule 2.
        if question.id().is_empty() {
            return Err(Invalid { question: String::new(), reason: "a question's id is empty" });
        }
        if !ids.insert(question.id()) {
            return Err(invalid(question, "the id is another question's too"));
        }
        // Rule 3.
        if !real(question.instructions()) {
            return Err(invalid(question, "the instructions are empty"));
        }
        match question {
            Question::Noul { .. } => {}
            // Rule 4.
            Question::Choice { options, .. } => {
                if options.len() < 2 {
                    return Err(invalid(question, "a choice has at least two options"));
                }
                let mut values = HashSet::new();
                for option in options {
                    if option.value.is_empty() {
                        return Err(invalid(question, "an option's value is empty"));
                    }
                    if !values.insert(option.value.as_str()) {
                        return Err(invalid(question, "two options have the same value"));
                    }
                    if option.description.as_deref().is_some_and(|description| !real(description)) {
                        return Err(invalid(question, "an option's description is present and empty"));
                    }
                }
            }
            // Rule 5.
            Question::Score { levels, .. } => {
                if levels.len() < 2 {
                    return Err(invalid(question, "a score has at least two levels"));
                }
                if levels.iter().any(|level| !real(level)) {
                    return Err(invalid(question, "a level is empty"));
                }
            }
        }
    }
    Ok(())
}
