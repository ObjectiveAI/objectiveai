//! A hook that judges the credential.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A hook that judges the credential itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    /// The hook, by name: the folder `hooks/<name>/` of the
    /// provider's directory, run as [`hook`](crate::hook) provides.
    /// It reads a [`HookInput`] and writes a [`HookOutput`].
    pub authorize_hook: String,
}

/// What the hook receives on stdin, as one line of JSON.
///
/// ```json
/// {"credential": "5f1c…", "address": "203.0.113.7"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookInput {
    /// The credential the peer presented, as it was presented.
    pub credential: String,
    /// The peer's address, as the OS reported it: `203.0.113.7`, or
    /// `2001:db8::7`.
    pub address: IpAddr,
}

/// What the hook writes to stdout, as one JSON document, on exit `0`.
///
/// An object whose `authorized` is `true` or `false`, and which
/// carries `identity` exactly when it is `true`. `true` without an
/// identity, `false` with one, or any other key, is refused as not
/// this type. Anything else — a non-zero exit, a stdout that is not
/// this — is a [`hook::Error`](crate::hook::Error): the hook has not
/// answered, and the credential is refused.
///
/// ```json
/// {"authorized": true, "identity": "acme"}
/// ```
///
/// ```json
/// {"authorized": false}
/// ```
///
/// Serde has no boolean tag, so the discriminating is written out:
/// the document is read as its two fields and the pair is judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOutput {
    /// The credential is accepted, and this is who presented it: the
    /// string every handler and every capability receives as the
    /// client's identity.
    Authorized {
        /// The peer's identity.
        identity: String,
    },
    /// The credential is refused, and nothing of that reaches the
    /// peer.
    Refused,
}

/// The document as written: the two fields, before the pair is
/// judged.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fields {
    authorized: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    identity: Option<String>,
}

impl Serialize for HookOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let fields = match self {
            HookOutput::Authorized { identity } => Fields {
                authorized: true,
                identity: Some(identity.clone()),
            },
            HookOutput::Refused => Fields {
                authorized: false,
                identity: None,
            },
        };
        fields.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HookOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let fields = Fields::deserialize(deserializer)?;
        match (fields.authorized, fields.identity) {
            (true, Some(identity)) => Ok(HookOutput::Authorized { identity }),
            (true, None) => Err(D::Error::missing_field("identity")),
            (false, None) => Ok(HookOutput::Refused),
            (false, Some(_)) => Err(D::Error::custom(
                "`identity` is present but `authorized` is false",
            )),
        }
    }
}
