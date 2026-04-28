/// Result of a Codex CLI install attempt. Mirrors `CodexInstallResult` in
/// `install.py:19-23` (`@dataclass(frozen=True)`).
///
/// `installed` is `false` if the binary was already present and the call
/// returned without re-downloading; `true` when a fresh install happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexInstallResult {
    pub codex_path: String,
    pub installed: bool,
}
