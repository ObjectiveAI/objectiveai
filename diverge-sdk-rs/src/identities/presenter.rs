//! One identity presented, and nobody judged.

use std::net::IpAddr;

use futures_util::future::BoxFuture;

use super::claims::claims;
use super::{Identities, Name, VALID_FOR};
use crate::wire::handshake::{Authenticator, Peer};

/// The handshake's side of a program that dials as one identity and
/// judges nobody — see the [rules](super), rule 11:
///
/// 1. **Presenting.** At the identity's issuer, its access token, good
///    for at least [`VALID_FOR`], refreshed as rule 4 says. At any
///    other issuer, nothing: the handshake fails.
/// 2. **Taking the answer at its word.** The other end's token is read
///    for its `iss` and its `sub`, and neither its signature nor who it
///    names is judged. A token that is no JWT, or names no `iss` or no
///    `sub`, fails the handshake; the handshake itself still requires
///    the answer's issuer to be the one presented.
#[derive(Debug, Clone)]
pub struct Presenter {
    identities: Identities,
    name: Name,
}

impl Presenter {
    /// `name`, presented from `identities`.
    pub fn new(identities: Identities, name: Name) -> Presenter {
        Presenter { identities, name }
    }
}

impl Authenticator for Presenter {
    fn present<'a>(&'a self, issuer: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            if issuer != self.name.issuer {
                return Err(format!("this end presents {} and holds no identity at `{issuer}`", self.name));
            }
            let token = self.identities.access_token(&self.name, VALID_FOR).await.map_err(|error| error.to_string())?;
            Ok(token.expose().to_string())
        })
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Peer, String>> {
        Box::pin(async move {
            let read = claims(token).ok_or_else(|| "the other end's token is no JWT".to_string())?;
            match (read.iss, read.sub) {
                (Some(issuer), Some(subject)) => Ok(Peer { issuer, subject }),
                _ => Err("the other end's token names no issuer or no subject".to_string()),
            }
        })
    }
}
