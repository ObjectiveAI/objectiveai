//! A credential document the run needs from the vault, and where it
//! goes on disk.

/// One rotating document: its well-known vault key, and the file it
/// is written to before the gateway starts and read back from after
/// it exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Document {
    /// The vault key the document lives under, one of the SDK's
    /// well-known names.
    pub key: &'static str,
    /// Where the document lands.
    pub target: Target,
}

/// Where a credential document is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The `providers.<name>` entry of `auth.json`, verbatim.
    AuthEntry(&'static str),
    /// The Qwen CLI's own token file at the real home, verbatim.
    QwenCreds,
}
