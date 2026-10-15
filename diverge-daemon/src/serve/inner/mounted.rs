//! What a check of a create's mounts came to.

/// The mounts checked, or why they were not taken: as [`Checked`]
/// answers, with the one answer only a mount can have — a provider
/// named whose capabilities exclude what the mount asks of it.
///
/// [`Checked`]: super::Checked
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mounted {
    /// Every mount is of a volume its provider lists, served as the
    /// mount asks, and reached by the caller's grant.
    Ok,
    /// The caller holds no `mount` grant reaching a volume named.
    Forbidden,
    /// A volume named is not there, in a sentence: the create's
    /// error.
    Error(String),
    /// A provider named holds out no capability for what a mount
    /// asks of it: the create's `unsupported` error, in a sentence.
    Unsupported(String),
}
