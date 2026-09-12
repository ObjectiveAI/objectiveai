//! The command itself.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A command the provider runs to ask a question of the operator's
/// own program: an argv array, and no shell. See [the module](crate::hook)
/// for the convention it is run under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hook(pub Vec<String>);

impl Hook {
    /// The program: the first element, or `None` for an empty array,
    /// which the configuration loader refuses.
    pub fn program(&self) -> Option<&str> {
        self.0.first().map(String::as_str)
    }

    /// The arguments: every element after the first.
    pub fn args(&self) -> &[String] {
        self.0.get(1..).unwrap_or(&[])
    }

    /// Where the program is: an absolute first element as it is; else
    /// `hooks_dir/<program>` when that exists; else the bare name,
    /// which the OS resolves on `PATH`. `None` for an empty array.
    pub(super) fn resolve(&self, hooks_dir: &Path) -> Option<PathBuf> {
        let program = Path::new(self.program()?);
        if program.is_absolute() {
            return Some(program.to_path_buf());
        }
        let beside = hooks_dir.join(program);
        if beside.exists() {
            return Some(beside);
        }
        Some(program.to_path_buf())
    }
}
