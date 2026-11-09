//! Whether a party may be listed by a provider's `daemons::list`, and
//! what it says of itself there.

use super::description::Description;

/// What a party's authorization frame says of listing it: not at all,
/// or listed by every provider it is connected to, with or without a
/// [`Description`]. Only a provider honors it; every other end reads it
/// and ignores it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Listable {
    /// Not listed. The flag `0`.
    #[default]
    Hidden,
    /// Listed, with what it says of itself, if anything: the flag `1`
    /// without a description, `2` with one.
    Listed(Option<Description>),
}

impl Listable {
    /// Whether the party is listed.
    pub fn is_listed(&self) -> bool {
        matches!(self, Listable::Listed(_))
    }
}
