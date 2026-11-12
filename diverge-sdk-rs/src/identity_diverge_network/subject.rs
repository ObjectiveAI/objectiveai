//! A subject of `identity.diverge.network`, read.

use super::issuer::ISSUERS;
use super::{Account, Error};

/// The account that owns `subject` at `identity.diverge.network`, and
/// the subject's name: `<issuer>.<id>.<name>`, read. The rules:
///
/// 1. **Split at the first two periods**, and no other. The issuer
///    and the id hold none, and the name is everything after the
///    second, periods and all.
/// 2. **The issuer is one there is** — see
///    [`ISSUERS`](super::ISSUERS).
/// 3. **The id is whatever it is.** It is not held to digits, nor to
///    anything but being there: what an issuer's ids look like is the
///    issuer's.
/// 4. **None of the three is empty.**
///
/// A subject that is not so is [`Error::Subject`]: no subject of
/// `identity.diverge.network`'s.
pub fn owner(subject: &str) -> Result<(Account, String), Error> {
    let invalid = || Error::Subject(subject.to_string());
    // Rule 1.
    let (issuer, rest) = subject.split_once('.').ok_or_else(invalid)?;
    let (id, name) = rest.split_once('.').ok_or_else(invalid)?;
    // Rules 2 and 4.
    if !ISSUERS.iter().any(|known| known.name == issuer) || id.is_empty() || name.is_empty() {
        return Err(invalid());
    }
    Ok((Account { issuer: issuer.to_string(), id: id.to_string() }, name.to_string()))
}
