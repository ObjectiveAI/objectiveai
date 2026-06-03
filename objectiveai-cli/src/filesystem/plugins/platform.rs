use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A supported runtime target — the cross product of OS and CPU
/// architecture the cli knows how to install plugin binaries for.
/// Serialized as `<os>_<arch>` (e.g. `"linux_x86_64"`,
/// `"windows_aarch64"`). Used as the key type in
/// [`super::Manifest::binaries`] so a manifest can declare a distinct
/// release-asset filename per platform. The underscore separator (vs
/// the hyphen used by Rust target triples) keeps the names usable
/// directly as identifiers in the cross-language SDK codegen.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "filesystem.plugins.Platform")]
pub enum Platform {
    #[serde(rename = "linux_x86_64")]
    LinuxX86_64,
    #[serde(rename = "linux_aarch64")]
    LinuxAarch64,
    #[serde(rename = "windows_x86_64")]
    WindowsX86_64,
    #[serde(rename = "windows_aarch64")]
    WindowsAarch64,
    #[serde(rename = "macos_x86_64")]
    MacosX86_64,
    #[serde(rename = "macos_aarch64")]
    MacosAarch64,
}

impl Platform {
    /// The platform this binary was built for, if recognized. Returns
    /// `None` on exotic build targets (BSD, RISC-V, 32-bit ARM, etc.)
    /// — those simply have no manifest binding.
    pub fn current() -> Option<Self> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Some(Self::LinuxX86_64),
            ("linux", "aarch64") => Some(Self::LinuxAarch64),
            ("windows", "x86_64") => Some(Self::WindowsX86_64),
            ("windows", "aarch64") => Some(Self::WindowsAarch64),
            ("macos", "x86_64") => Some(Self::MacosX86_64),
            ("macos", "aarch64") => Some(Self::MacosAarch64),
            _ => None,
        }
    }
}
