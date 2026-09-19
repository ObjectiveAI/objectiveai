//! `hook.yaml`: how a hook is run, per platform.

use serde::{Deserialize, Serialize};

/// The manifest's file name inside a hook's folder.
pub const MANIFEST: &str = "hook.yaml";

/// The command per platform, each optional, no default.
///
/// Each entry is a whole argv array: the program first, then its
/// arguments, exactly as the OS receives them. An entry replaces
/// nothing and merges with nothing; the running platform's entry is
/// the command, and a manifest without one does not run here.
///
/// ```yaml
/// windows: ["py", "-3", "authorize.py"]
/// macos:   ["python3", "authorize.py"]
/// linux:   ["python3", "authorize.py"]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Manifest {
    /// The command on Windows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows: Option<Vec<String>>,
    /// The command on macOS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macos: Option<Vec<String>>,
    /// The command on Linux.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linux: Option<Vec<String>>,
}

impl Manifest {
    /// This platform, as the manifest spells it: the key its command
    /// is under.
    pub const PLATFORM: &str = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    /// This platform's command, or `None` when the manifest has none.
    pub fn command(&self) -> Option<&[String]> {
        let entry = if cfg!(target_os = "windows") {
            &self.windows
        } else if cfg!(target_os = "macos") {
            &self.macos
        } else {
            &self.linux
        };
        entry.as_deref()
    }
}
