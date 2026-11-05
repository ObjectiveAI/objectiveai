//! Signing an identity in by its password: the Authorization Code login,
//! its credentials sent by this end.

use crate::identities::{Error, Identities, Name};

impl Identities {
    /// Sign `name` in by its `password`, as the public client
    /// `client_id`, asking for `scope` and, where given, for an access
    /// token for `resource`: OpenID Connect's Authorization Code login —
    /// see [`login_code`](Self::login_code) — with the subject and the
    /// password sent as Basic credentials to the authorization endpoint
    /// and its redirect read rather than followed — see
    /// [`with_password`](super::Authorizing::with_password). No browser,
    /// and no person. The identity's lock is taken first, waiting while
    /// another holds it. See the [rules](crate::identities), rule 12.
    pub async fn login_password(&self, name: &Name, client_id: &str, password: &str, scope: &str, resource: Option<&str>) -> Result<Name, Error> {
        self.login_code(name, client_id, scope, resource).await?.with_password(password).await
    }

    /// [`login_password`](Self::login_password), the lock taken only if
    /// nobody holds it: [`Error::Held`] at once if another login or a
    /// reader of the identity holds it, and nothing asked of the issuer.
    pub async fn try_login_password(&self, name: &Name, client_id: &str, password: &str, scope: &str, resource: Option<&str>) -> Result<Name, Error> {
        self.try_login_code(name, client_id, scope, resource).await?.with_password(password).await
    }
}
