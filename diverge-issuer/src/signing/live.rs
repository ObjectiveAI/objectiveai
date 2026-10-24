//! The keys as a running issuer holds them.

use super::{Error, Signing};

/// The signing keys a running issuer signs and publishes with: the keys
/// themselves, and the key set made of them, as the JSON `/jwks`
/// answers. Made whole at once, so the key set is always the keys'.
pub struct Live {
    /// The keys.
    pub signing: Signing,
    /// The key set, as JSON.
    pub key_set: String,
}

impl Live {
    /// `signing`, with its key set made.
    pub fn new(signing: Signing) -> Result<Live, Error> {
        let key_set = serde_json::to_string(&signing.key_set()).map_err(Error::KeySet)?;
        Ok(Live { signing, key_set })
    }
}
