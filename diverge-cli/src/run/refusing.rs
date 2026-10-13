//! An authorizer that is never asked.

use std::fmt;
use std::net::IpAddr;

use diverge_sdk::wire::client::unbrokered_authorizer::UnbrokeredAuthorizer;

/// The judge of a credential on a connection the CLI dialled: the
/// SDK's handshake takes one for either direction, and on an outgoing
/// connection it is never called, since the CLI presents the
/// credential and judges nothing. Were it called, it would refuse.
#[derive(Debug, Clone, Copy, Default)]
pub struct Refusing;

/// The refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused;

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a credential was presented on a connection the CLI dialled")
    }
}

impl UnbrokeredAuthorizer for Refusing {
    type Error = Refused;

    async fn authorize(&self, _: &str, _: IpAddr) -> Result<String, Refused> {
        Err(Refused)
    }
}
