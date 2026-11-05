//! Signing an identity in by its password: OAuth's password grant.

use zeroize::Zeroizing;

use super::device::scoped;
use crate::file_lock;
use crate::identities::discovery::discover;
use crate::identities::identities::folder_owned;
use crate::identities::oauth::{Answer, Tokens, post};
use crate::identities::record::{Record, write};
use crate::identities::{Error, Identities, Name};

impl Identities {
    /// Sign `name` in by its `password`, as the public client
    /// `client_id`, asking for `scope` — `openid` and `offline_access`
    /// added where it lacks them — and, where given, for an access token
    /// for `resource`: OAuth's resource owner password credentials grant,
    /// RFC 6749 §4.3, one round trip. The identity's lock is taken
    /// first, waiting while another holds it. See the
    /// [rules](crate::identities), rule 12.
    pub async fn login_password(&self, name: &Name, client_id: &str, password: &str, scope: &str, resource: Option<&str>) -> Result<Name, Error> {
        let paths = self.paths(name);
        folder_owned(&paths.folder).await?;
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        self.grant(held, name, client_id, password, scope, resource).await
    }

    /// [`login_password`](Self::login_password), the lock taken only if
    /// nobody holds it: [`Error::Held`] at once if another login or a
    /// reader of the identity holds it, and nothing asked of the issuer.
    pub async fn try_login_password(&self, name: &Name, client_id: &str, password: &str, scope: &str, resource: Option<&str>) -> Result<Name, Error> {
        let paths = self.paths(name);
        folder_owned(&paths.folder).await?;
        let Some(held) = file_lock::try_exclusive(&paths.lock).await.map_err(Error::Lock)? else {
            return Err(Error::Held(name.clone()));
        };
        self.grant(held, name, client_id, password, scope, resource).await
    }

    /// The grant, the identity's lock in hand and held to the end: the
    /// discovery document read, the token endpoint asked, and the
    /// identity written — replacing one already there, signed out or
    /// not — when the answer is for the identity named. A refusal as
    /// `invalid_grant` is [`Error::Rejected`]; any other is
    /// [`Error::Refused`]; an answer for another identity is
    /// [`Error::OtherSubject`], and nothing is written.
    async fn grant(&self, held: file_lock::Held, name: &Name, client_id: &str, password: &str, scope: &str, resource: Option<&str>) -> Result<Name, Error> {
        let discovery = discover(&self.http, &name.issuer).await?;
        let scope = scoped(scope);
        let password = Zeroizing::new(password.to_string());
        let mut form = vec![
            ("grant_type", "password"),
            ("client_id", client_id),
            ("username", name.subject.as_str()),
            ("password", password.as_str()),
            ("scope", scope.as_str()),
        ];
        if let Some(resource) = resource {
            form.push(("resource", resource));
        }
        let tokens = match post::<Tokens>(&self.http, &discovery.token_endpoint, &form).await? {
            Answer::Answered(tokens) => tokens,
            Answer::Refused(refusal) if refusal.error == "invalid_grant" => return Err(Error::Rejected),
            Answer::Refused(refusal) => return Err(Error::Refused { code: refusal.error, description: refusal.error_description }),
        };
        let record = Record::signed_in(&discovery.issuer, client_id, &discovery.token_endpoint, &scope, resource, tokens)?;
        let answered = record.name();
        if &answered != name {
            return Err(Error::OtherSubject { expected: name.clone(), answered });
        }
        write(&self.paths(&answered), &record).await?;
        drop(held);
        Ok(answered)
    }
}
