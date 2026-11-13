//! A structured decision: a prompt and typed questions, answered with
//! numbers.
//!
//! Beside chat, the other thing an agent may be asked. A
//! [`Decision`] is content, as a chat message's is, and one or more
//! [`Question`]s, each of one of three kinds — a choice among options,
//! a score along levels, or a noul, a single number. The agent's
//! program answers every question at once with an
//! [`Answer`] apiece, under the question's id.
//!
//! The two share one order toward the agent and run apart. A chat
//! message joins a loop that is running; a decision waits for the
//! agent's program to be idle, starts a run of its own, and is waited
//! for to that run's end before anything else — chat or decision — is
//! handed over. The container proxy keeps that order, by the `index`
//! every enqueue of either kind carries: see
//! [`enqueue`](crate::shared::containers::enqueue).
//!
//! [`validate`] is what a caller's questions are held to before they
//! are sent on, and
//! [`StructuredDecisions::lacking`](crate::shared::containers::info::StructuredDecisions::lacking)
//! what their content is held to.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod answer;
mod choice_option;
mod decision;
mod question;
mod validate;

pub use answer::*;
pub use choice_option::*;
pub use decision::*;
pub use question::*;
pub use validate::*;
