//! A resource the plan needs fetched, and where its document goes.

/// One resource to fetch before anything is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// The request field that named it, for the error that says
    /// which one could not be had.
    pub field: &'static str,
    /// The resource's identity, the FILE grammar.
    pub identity: String,
    /// Where the fetched document lands.
    pub target: Target,
}

/// Where a fetched resource document is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The `providers.<name>` entry of `auth.json`, verbatim.
    AuthEntry(&'static str),
    /// The Qwen CLI's own token file at the real home, verbatim.
    QwenCreds,
}
