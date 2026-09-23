//! Who a provider is, by how it came to be connected.

use serde::{Deserialize, Serialize};

/// A provider's identity: one of the ways the daemon came to be
/// connected to it, and what that way establishes.
///
/// One object, told apart by `kind`, because the two ways establish
/// different things — an address the daemon chose, or a string its
/// own judging answered — and a third is coming. The `kind` mirrors
/// the mode byte of the protocol's credential, and for the same
/// reason: a discriminator added later is a wire break, and one
/// present from the start costs a reader nothing.
///
/// # `brokered` is reserved
///
/// A provider that dials the daemon with a brokered credential — a
/// third party vouching for it, the credential mode the protocol
/// reserves and does not yet define — will be a third variant here,
/// `kind: "brokered"`, when `diverge-broker-sdk` defines the mode.
/// Nothing about its shape is guessed at now; the `kind` is there so
/// that it can land beside these two without moving either.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Identity {
    /// The daemon dialled the provider. `kind: "outgoing"`.
    ///
    /// The address is the identity, because the daemon chose whom to
    /// dial and that choice IS the identification — the same rule
    /// the provider applies to a peer it dials. Nothing on the wire
    /// told the daemon anything it did not already know.
    #[serde(rename = "outgoing")]
    Outgoing {
        /// Where the provider was dialled: `host:port`, as the
        /// daemon's configuration writes it — the host a name the
        /// resolver answers for or an IP address, `[…]:port` for an
        /// IPv6 address.
        address: String,
    },
    /// The provider dialled the daemon and presented an unbrokered
    /// credential. `kind: "unbrokered"`.
    ///
    /// The identity is what the daemon's judging of the credential
    /// answered: the string a key names, or a hook returns — the same
    /// form a provider identifies its own connectors by, the daemon
    /// being the provider's mirror in this. The credential itself is
    /// never here; it is a secret, and the identity is what it
    /// established.
    #[serde(rename = "unbrokered")]
    Unbrokered {
        /// The provider's identity, as the daemon's judging of its
        /// credential answered.
        identity: String,
    },
}
