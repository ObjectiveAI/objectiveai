//! A command the operator supplies, run to answer a question.

use serde::{Deserialize, Serialize};

/// A command the provider runs to ask a question of the operator's
/// own program: an argv array, and no shell.
///
/// # The program
///
/// The first element is the program. An absolute path is run as it
/// is; a bare name or a relative path resolves first against the
/// provider's `hooks/` directory, then on `PATH`. The process runs
/// with `hooks/` as its working directory. An empty array is refused
/// when the configuration is loaded.
///
/// # The question
///
/// Nothing is appended to argv: the elements after the first are the
/// arguments, exactly as written. The question travels as one JSON
/// document on stdin, and the identity it concerns also travels as
/// `DIVERGE_PROVIDER_IDENTITY` in the environment — never in argv,
/// which every user of the machine can read.
///
/// # The answer
///
/// Exit status `0` is yes. Every other status, a program that could
/// not be started, and a program that dies are no. For a yes-or-no
/// question stdout is ignored; stderr is logged. There is no timeout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hook(pub Vec<String>);

impl Hook {
    /// The program: the first element, or `None` for an array the
    /// loader would have refused.
    pub fn program(&self) -> Option<&str> {
        self.0.first().map(String::as_str)
    }

    /// The arguments: every element after the first.
    pub fn args(&self) -> &[String] {
        self.0.get(1..).unwrap_or(&[])
    }
}
