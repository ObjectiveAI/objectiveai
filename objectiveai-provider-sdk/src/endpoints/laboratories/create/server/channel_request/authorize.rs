//! A connector asking to attach.

use std::net::IpAddr;

/// Who wants in, and what they offer.
///
/// # Two kinds of claim, and they are not equal
///
/// [`address`](Self::address) is ATTESTED — the provider saw it on a
/// socket and is vouching for it. [`authorization`](Self::authorization)
/// is ASSERTED — the connector wrote it and the provider passed it on
/// without looking.
///
/// They are separate fields rather than one blob so that a creator
/// cannot confuse them by accident. Treating an asserted value as an
/// attested one is not a bug in the ordinary sense; it is the whole of
/// how this kind of check gets defeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Authorize<'a> {
    /// Where the connector's socket comes from.
    ///
    /// What the PROVIDER sees, which is not always where the connector
    /// is. NAT and carrier-grade NAT collapse many peers onto one
    /// address; a provider behind a load balancer sees the balancer.
    /// It is a signal, not an identity, and a creator that treats it
    /// as one is trusting the provider's network topology to stay the
    /// shape it is today.
    ///
    /// A real address rather than a string: `::1` and
    /// `0:0:0:0:0:0:0:1` are one address written two ways, and a
    /// creator comparing text would call them different.
    pub address: IpAddr,
    /// Whatever the creator needs in order to say yes.
    ///
    /// The connector's
    /// [`authorization`](crate::endpoints::laboratories::connect::client::request::Frame::authorization),
    /// relayed verbatim. Opaque — a shared secret, a signed token, a
    /// name — and the provider neither reads it nor could usefully:
    /// what makes one connector acceptable is something only a creator
    /// knows.
    ///
    /// May be empty, which is a connector offering nothing. Whether
    /// that is ever enough is the creator's to decide.
    pub authorization: &'a [u8],
}
