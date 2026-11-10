//! Signing an identity in on the daemon's host.

use diverge_sdk::daemon::endpoints::identities::login::client::request;
use diverge_sdk::daemon::endpoints::identities::login::server::response::{Frame, Pending};
use diverge_sdk::daemon::grant::identities::{Make, Over};
use diverge_sdk::identities::{Error, Name};
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{files, reply};
use crate::store;

/// Serve the login and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    if let Err(error) = serve(&scope, frame, who, daemon).await {
        reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
    }
    scope.send_response_finish().await;
}

/// The rules:
///
/// 1. **Judged.** An identity not on the host needs the making `login`;
///    one already there, a grant of `login` that reaches it, by its tags
///    among the rest. Else one `Forbidden`.
/// 2. **Never waiting.** The identity's lock is taken only if nobody
///    holds it: `Held` at once if another login or a reader does, and
///    nothing asked of the issuer. A login that cannot begin otherwise —
///    the issuer unreached, its document lacking a device endpoint, its
///    refusal — is the one `Error`.
/// 3. **By password**, where the request gives one: the Authorization
///    Code login with no browser, on this host — the password the
///    authorization request's Basic credentials, the redirect read — and
///    exactly one of `SignedIn`, `Rejected` for a subject and password
///    the issuer refused, `OtherSubject` or `Error`; no `Pending`.
///    The rules below are the device login's.
/// 4. **The code.** Begun, exactly one `Pending`: the user code and where
///    to enter it, never the device code.
/// 5. **The outcome.** The issuer polled until the person decides or the
///    code expires: `SignedIn` once the identity is written — replacing
///    one already there — `Denied`, `Expired`, `OtherSubject` for an
///    approval as another identity, which writes nothing, or `Error`.
/// 6. **A cancel**, or the client's connection ending, ends the poll:
///    the login dropped, its lock let go, nothing written, and no
///    outcome sent.
/// 7. **Not presented.** Which identity the daemon is stays its
///    configuration's: a new identity is the host's, and only that.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) -> Result<(), store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    // Rule 1.
    let Some(standing) = standing else {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    };
    let name = Name { issuer: frame.issuer, subject: frame.subject };
    let allowed = {
        let mut conn = daemon.store.acquire().await?;
        match super::held(&mut conn, daemon).await? {
            Ok(identities) => match identities.iter().find(|identity| identity.issuer == name.issuer && identity.subject == name.subject) {
                Some(identity) => judge::identities::over(&standing, Over::Login, identity),
                None => judge::identities::make(&standing, Make::Login),
            },
            Err(error) => {
                reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
                return Ok(());
            }
        }
    };
    if !allowed {
        reply::reply(scope, &Frame::Forbidden).await;
        return Ok(());
    }
    // Rule 2.
    let identities = daemon.authenticator.identities();
    // Rule 3.
    if let Some(password) = frame.password.as_deref() {
        let asked = frame.scope.as_deref().unwrap_or("");
        let answer = match identities.try_login_password(&name, &frame.client_id, password, asked, frame.resource.as_deref()).await {
            Ok(name) => Frame::SignedIn(identity(name)),
            Err(Error::Held(_)) => Frame::Held,
            Err(Error::Rejected) => Frame::Rejected,
            Err(Error::OtherSubject { answered, .. }) => Frame::OtherSubject(identity(answered)),
            Err(error) => Frame::Error(reply::failure(&error)),
        };
        reply::reply(scope, &answer).await;
        return Ok(());
    }
    let pending = match identities.try_login(&name, &frame.client_id, frame.scope.as_deref().unwrap_or(""), frame.resource.as_deref()).await {
        Ok(pending) => pending,
        Err(Error::Held(_)) => {
            reply::reply(scope, &Frame::Held).await;
            return Ok(());
        }
        Err(error) => {
            reply::reply(scope, &Frame::Error(reply::failure(&error))).await;
            return Ok(());
        }
    };
    // Rule 3.
    reply::reply(
        scope,
        &Frame::Pending(Pending {
            user_code: pending.user_code.clone(),
            verification_uri: pending.verification_uri.clone(),
            verification_uri_complete: pending.verification_uri_complete.clone(),
            expires_in: pending.expires_in,
        }),
    )
    .await;
    // Rules 4 and 5.
    let outcome = tokio::select! {
        outcome = pending.wait() => outcome,
        () = files::cancelled(scope) => return Ok(()),
    };
    let answer = match outcome {
        Ok(name) => Frame::SignedIn(identity(name)),
        Err(Error::Denied) => Frame::Denied,
        Err(Error::Expired) => Frame::Expired,
        Err(Error::OtherSubject { answered, .. }) => Frame::OtherSubject(identity(answered)),
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(scope, &answer).await;
    Ok(())
}

/// An identity's name, as the wire carries one.
fn identity(name: Name) -> Identity {
    Identity { issuer: name.issuer, subject: name.subject }
}
