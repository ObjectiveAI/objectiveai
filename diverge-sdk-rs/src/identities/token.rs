//! A token, handed out.

use std::fmt;

use zeroize::Zeroizing;

/// An access token or an ID token, good until [`expires`](Self::expires).
///
/// It has no way out but [`expose`](Self::expose): no `Serialize`, no
/// `Display`, and a `Debug` that prints `Token(<hidden>)`. Its memory
/// is zeroed when it is dropped.
#[derive(Clone)]
pub struct Token {
    value: Zeroizing<String>,
    expires: i64,
}

impl Token {
    /// The token `value`, good until `expires`.
    pub(super) fn new(value: &str, expires: i64) -> Token {
        Token { value: Zeroizing::new(value.to_string()), expires }
    }

    /// The token itself, for the wire.
    pub fn expose(&self) -> &str {
        &self.value
    }

    /// When it stops being good, in seconds since the Unix epoch.
    pub fn expires(&self) -> i64 {
        self.expires
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<hidden>)")
    }
}
