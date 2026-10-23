//! An authorizer that is never asked.

use std::fmt;
use std::net::IpAddr;

use super::unbrokered_authorizer::UnbrokeredAuthorizer;

/// The judge of a credential on a connection this end dialled.
///
/// The handshake takes an authorizer for either direction, and on an
/// outgoing connection it is never called, since this end presents
/// the credential and judges nothing. Were it called, it would refuse.
/// What [`dial`](super::dial::dial) presents with.
#[derive(Debug, Clone, Copy, Default)]
pub struct Refusing;

/// The refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused;

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a credential was presented on a connection this end dialled")
    }
}

impl std::error::Error for Refused {}

impl UnbrokeredAuthorizer for Refusing {
    type Error = Refused;

    async fn authorize(&self, _: &str, _: IpAddr) -> Result<String, Refused> {
        Err(Refused)
    }
}
