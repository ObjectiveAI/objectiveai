//! What a subject may be.

use super::Error;

/// The longest a subject may be, which is OpenID
/// Connect's bound on `sub`.
const LONGEST: usize = 255;

/// Whether `subject` may name an identity: 1 to 255 printable ASCII
/// characters, no space among them, since it is the `sub` of every
/// token issued to the identity.
pub fn subject(subject: &str) -> Result<(), Error> {
    if subject.is_empty() || subject.len() > LONGEST || !subject.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(Error::Invalid(format!(
            "`{subject}` is not a subject: 1 to {LONGEST} printable ASCII characters, with no space"
        )));
    }
    Ok(())
}

/// Whether `error` is the database refusing a row whose key another
/// row holds.
pub(super) fn taken(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation())
}
