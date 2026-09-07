//! A connector asking to attach.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// Who wants in, and what they offer.
///
/// # Two kinds of claim, and they are not equal
///
/// [`address`](Self::address) is ATTESTED — the provider saw it on a
/// socket and is vouching for it. [`authorization`](Self::authorization)
/// is ASSERTED — the connector wrote it and the provider passed it on
/// without looking.
///
/// They are separate fields rather than one blob so that a runner
/// cannot confuse them by accident. Treating an asserted value as an
/// attested one is not a bug in the ordinary sense; it is the whole of
/// how this kind of check gets defeated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Authorize {
    /// Where the connector's socket comes from.
    ///
    /// What the PROVIDER sees, which is not always where the connector
    /// is. NAT and carrier-grade NAT collapse many peers onto one
    /// address; a provider behind a load balancer sees the balancer.
    /// It is a signal, not an identity, and a runner that treats it
    /// as one is trusting the provider's network topology to stay the
    /// shape it is today.
    ///
    /// An [`IpAddr`] rather than a string, even though JSON carries it
    /// as one. `::1` and `0:0:0:0:0:0:0:1` are one address written two
    /// ways, and a runner comparing text would call them different —
    /// parsing it into an address is what makes the comparison the
    /// right one, and doing that here means every runner gets it
    /// rather than each remembering to.
    pub address: IpAddr,
    /// Whatever the runner needs in order to say yes.
    ///
    /// The connector's
    /// [`authorization`](crate::shared::containers::request::Connect::authorization),
    /// relayed verbatim. Opaque — a shared secret, a signed token, a
    /// name — and the provider neither reads it nor could usefully:
    /// what makes one connector acceptable is something only a runner
    /// knows.
    ///
    /// Owned rather than borrowed from the frame, because a JSON
    /// string with an escape in it is not a slice of the bytes it
    /// arrived in. A credential is unlikely to carry one and the
    /// protocol is not going to promise it does not.
    ///
    /// May be empty, which is a connector offering nothing. Whether
    /// that is ever enough is the runner's to decide.
    pub authorization: String,
}
