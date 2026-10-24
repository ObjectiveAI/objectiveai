//! Which identity, and where it is.

use std::fmt;
use std::path::PathBuf;

use super::Identities;
use super::encode::encode;

/// An identity: one subject at one issuer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Name {
    /// The issuer identifier, exactly as its discovery document gives
    /// it: every token's `iss`.
    pub issuer: String,
    /// The subject: every token's `sub`.
    pub subject: String,
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` at `{}`", self.subject, self.issuer)
    }
}

/// The files of one identity.
pub(super) struct Paths {
    /// The issuer's folder.
    pub(super) folder: PathBuf,
    /// The identity: `<subject>.json`.
    pub(super) record: PathBuf,
    /// Its lock: `<subject>.lock`.
    pub(super) lock: PathBuf,
    /// Where a write is staged before it is renamed over the record:
    /// `.<subject>.json.writing`.
    pub(super) staging: PathBuf,
}

impl Identities {
    /// The files of `name`, under the identities' folder.
    pub(super) fn paths(&self, name: &Name) -> Paths {
        let folder = self.folder.join(encode(&name.issuer));
        let subject = encode(&name.subject);
        Paths {
            record: folder.join(format!("{subject}.json")),
            lock: folder.join(format!("{subject}.lock")),
            staging: folder.join(format!(".{subject}.json.writing")),
            folder,
        }
    }
}
