//! A remote daemon's record, by what it reaches and as whom.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

use super::Routes;
use crate::daemon::endpoints::providers::Ip;
use crate::shared::Identity;
use crate::shared::canonical;

/// The base62 digits, in the order of their values.
const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// The most digits a `u64` takes in base62: `u64::MAX` is eleven.
const DIGITS: usize = 11;

/// A remote daemon's record, by the hash of what it reaches and as whom:
/// the one way every relay, every connected tool, every key, reference
/// and attestation, and every list filter and grant names a record. On
/// the wire a base62 string — `0-9A-Za-z`, the shortest form, one to
/// eleven characters. The rules of [`DaemonId::of`]:
///
/// 1. **What it covers.** `daemon`, `identity`, and the three filters of
///    the record's [`Routes`] — `providers`, `ips`, `addresses` — and
///    nothing else: no description, no tags, no creator, no time.
/// 2. **Absent is empty.** A filter not given hashes as the empty list,
///    as [`Routes`] treats the two alike: both are no filter.
/// 3. **Sorted and deduplicated.** Each filter is a set, not a sequence:
///    - `providers` by issuer, then subject;
///    - `ips` by their canonical JSON, each CIDR block normalised to its
///      network address first, so `10.0.0.7/24` is `10.0.0.0/24`;
///    - `addresses` as strings.
/// 4. **The digest.** SHA-256 over the
///    [canonical bytes](crate::shared::canonical::bytes) of the
///    normalised five, as one object; the ID is its first eight bytes,
///    big-endian.
/// 5. **What it buys.** The ID names the target. A record cannot be
///    changed to reach elsewhere, or as another account, under the same
///    ID — only its description and tags change — so a hop that
///    resolves an ID resolves exactly the record the caller meant, or
///    none.
///
/// # The string
///
/// The value in base62, most significant digit first, with no leading
/// zero but for `0` itself: each ID has exactly one spelling. A string
/// of any other character, empty, longer than eleven, with a leading
/// zero, or above `u64::MAX` is not an ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DaemonId(pub u64);

impl DaemonId {
    /// The ID of the record reaching `daemon` as `identity` by `routes`:
    /// rules 1 to 4.
    pub fn of(daemon: &Identity, identity: &Identity, routes: &Routes) -> DaemonId {
        // Rule 2: absent is empty. Rule 3: each a sorted set.
        let mut providers = routes.providers.clone().unwrap_or_default();
        providers.sort();
        providers.dedup();
        let mut ips: Vec<(Vec<u8>, Ip)> = routes.ips.clone().unwrap_or_default().into_iter().map(|ip| {
            let ip = match ip {
                Ip::Cidr { cidr } => Ip::Cidr { cidr: cidr.trunc() },
                ip => ip,
            };
            // An IP is plain data: its JSON cannot fail.
            (canonical::bytes(&ip).expect("an IP serializes"), ip)
        }).collect();
        ips.sort_by(|(left, _), (right, _)| left.cmp(right));
        ips.dedup_by(|(left, _), (right, _)| left == right);
        let ips: Vec<Ip> = ips.into_iter().map(|(_, ip)| ip).collect();
        let mut addresses = routes.addresses.clone().unwrap_or_default();
        addresses.sort();
        addresses.dedup();
        // Rule 4: the five as one object, canonical, hashed. Plain data:
        // its JSON cannot fail.
        let value = serde_json::json!({
            "daemon": daemon,
            "identity": identity,
            "providers": providers,
            "ips": ips,
            "addresses": addresses,
        });
        let digest = Sha256::digest(canonical::bytes(&value).expect("plain data serializes"));
        let mut first = [0u8; 8];
        first.copy_from_slice(&digest[..8]);
        DaemonId(u64::from_be_bytes(first))
    }
}

/// The base62 string: see [`DaemonId`].
impl fmt::Display for DaemonId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut digits = [0u8; DIGITS];
        let mut start = DIGITS;
        let mut rest = self.0;
        loop {
            start -= 1;
            digits[start] = ALPHABET[(rest % 62) as usize];
            rest /= 62;
            if rest == 0 {
                break;
            }
        }
        // Every byte is one of the alphabet's, so ASCII.
        f.write_str(std::str::from_utf8(&digits[start..]).expect("base62 digits are ASCII"))
    }
}

/// A string that is not an ID: see [`DaemonId`] for what one is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DaemonIdError {
    /// No characters.
    Empty,
    /// More than eleven characters.
    Long,
    /// A character that is no base62 digit.
    Digit,
    /// A leading zero on a value that is not zero: not the shortest form.
    LeadingZero,
    /// A value above `u64::MAX`.
    Overflow,
}

impl fmt::Display for DaemonIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonIdError::Empty => f.write_str("a daemon id is empty"),
            DaemonIdError::Long => write!(f, "a daemon id is more than {DIGITS} characters"),
            DaemonIdError::Digit => f.write_str("a daemon id holds a character that is not 0-9, A-Z or a-z"),
            DaemonIdError::LeadingZero => f.write_str("a daemon id has a leading zero"),
            DaemonIdError::Overflow => f.write_str("a daemon id is above the largest 64-bit value"),
        }
    }
}

impl std::error::Error for DaemonIdError {}

impl FromStr for DaemonId {
    type Err = DaemonIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return Err(DaemonIdError::Empty);
        }
        if bytes.len() > DIGITS {
            return Err(DaemonIdError::Long);
        }
        if bytes.len() > 1 && bytes[0] == b'0' {
            return Err(DaemonIdError::LeadingZero);
        }
        let mut value: u64 = 0;
        for byte in bytes {
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'A'..=b'Z' => byte - b'A' + 10,
                b'a'..=b'z' => byte - b'a' + 36,
                _ => return Err(DaemonIdError::Digit),
            };
            value = value.checked_mul(62).and_then(|value| value.checked_add(u64::from(digit))).ok_or(DaemonIdError::Overflow)?;
        }
        Ok(DaemonId(value))
    }
}

/// The base62 string.
impl Serialize for DaemonId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// The base62 string, and nothing else.
impl<'de> Deserialize<'de> for DaemonId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = Cow::<'de, str>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// A string of one to eleven base62 digits.
impl JsonSchema for DaemonId {
    fn schema_name() -> Cow<'static, str> {
        "DaemonId".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "diverge_sdk::daemon::endpoints::daemons::DaemonId".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^[0-9A-Za-z]{1,11}$",
            "description": "A remote daemon's record, by the hash of what it reaches and as whom: base62, the shortest form."
        })
    }
}
