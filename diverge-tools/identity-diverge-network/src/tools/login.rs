//! `login`: sign a subject in on the daemon's host, as an account.

use diverge_sdk::daemon::endpoints::identities::login::{client::execute, client::request, server::response};
use diverge_sdk::identity_diverge_network::{self, CLIENT_ID, ISSUER, SERVICE};
use futures_util::StreamExt as _;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::failure::Failure;
use crate::server::Call;
use crate::store;

/// The tool's name.
pub const NAME: &str = "login";

/// What the tool does, as its caller reads it.
pub const DESCRIPTION: &str = "Sign a subject of identity.diverge.network in on the daemon's host, as one of this tool's accounts, and wait for it. \
The subject signed in is `<issuer>.<id>.<subject>`; a name never used before begins a new subject. Nobody enters a code: the tool approves the login itself. \
Answers `{\"signed_in\": {\"issuer\", \"subject\"}}`.";

/// What `login` takes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// The account's issuer, as `list_accounts` gives it: `github` or
    /// `google`. Who issued the account's own sign-in, and never
    /// identity.diverge.network.
    pub issuer: String,
    /// The account's id at that issuer, as `list_accounts` gives it.
    pub id: String,
    /// The subject's name: its third part alone, 1 to 64 of `a-z`,
    /// `0-9`, `-` and `_`. The whole subject is
    /// `<issuer>.<id>.<subject>`.
    pub subject: String,
}

/// Sign `<issuer>.<id>.<subject>` in on the daemon's host. The rules:
///
/// 1. **The account is one the tool holds.** Its issuer is one there
///    is, else [`Failure::Issuer`]; it is kept under the tool's
///    template, else [`Failure::NoAccount`]. Neither the id nor the
///    name is empty.
/// 2. **Its token is had first** — see
///    [`token`](store::accounts::token), the one
///    `identity.diverge.network` takes of its issuer: under the
///    account's row lock, renewed where it has expired. An account that cannot approve
///    begins no login.
/// 3. **The daemon begins the login**: its `identities login`, of the
///    whole subject at [`ISSUER`], as the client [`CLIENT_ID`]. The
///    daemon's own host asks `identity.diverge.network` for the code
///    and polls it; the tool is served under its own grants, and a
///    daemon that grants it no login answers [`Failure::Forbidden`].
/// 4. **The tool approves the code** the daemon answers, as the
///    account, for the name — see
///    [`approve`](identity_diverge_network::approve). The code is one
///    this call began a moment before, and no other is ever approved.
/// 5. **Waited for.** The answers are read to the daemon's `signed_in`,
///    which is the tool's answer: the identity is on the daemon's
///    host. Any other ending is the failure, and nothing is signed in.
///    A failure here drops the answers' stream, which ends the login
///    on the daemon.
/// 6. **New or not, the same.** Whether the subject was ever signed in
///    before is not asked: a login under a name never used is how a
///    subject begins.
pub async fn call(call: Call<'_>, parameters: Parameters) -> Result<Value, Failure> {
    // Rule 1.
    let issuer = identity_diverge_network::issuer(&parameters.issuer).map_err(|error| Failure::Issuer(error.to_string()))?;
    if parameters.id.is_empty() || parameters.subject.is_empty() {
        return Err(Failure::Parameters("neither `id` nor `subject` may be empty".to_string()));
    }
    let subject = format!("{}.{}.{}", issuer.name, parameters.id, parameters.subject);
    // Rule 2.
    let pool = call.state.store.pool(&call.state.client).await?;
    let token = store::accounts::token(pool, &call.state.http, call.template, issuer, &parameters.id, issuer.presents).await?;
    // Rule 3.
    let request = request::Frame { issuer: ISSUER.to_string(), subject: subject.clone(), client_id: CLIENT_ID.to_string(), scope: None, resource: None, relay: None };
    let handle = call.state.daemon.handle(&call.state.client).await?;
    let (mut answers, _cancel) = match execute::execute(&handle, &request).await {
        Ok(opened) => opened,
        Err(error) => {
            call.state.daemon.drop_handle().await;
            return Err(Failure::Daemon(error.to_string()));
        }
    };
    // Rules 4 and 5.
    while let Some(answer) = answers.next().await {
        match answer.map_err(|error| Failure::Daemon(error.to_string()))? {
            response::Frame::Pending(pending) => {
                identity_diverge_network::approve(&call.state.http, SERVICE, issuer, &token, &pending.user_code, &parameters.subject).await.map_err(|error| Failure::Identity(error.to_string()))?;
            }
            response::Frame::SignedIn(identity) => return Ok(json!({ "signed_in": { "issuer": identity.issuer, "subject": identity.subject } })),
            response::Frame::Held => return Err(Failure::Held),
            response::Frame::Forbidden => return Err(Failure::Forbidden),
            response::Frame::Denied => return Err(Failure::Unapproved("denied")),
            response::Frame::Expired => return Err(Failure::Unapproved("expired")),
            response::Frame::OtherSubject(identity) => return Err(Failure::OtherSubject(identity.subject)),
            response::Frame::Error(error) => return Err(Failure::Daemon(error.0.to_string())),
        }
    }
    Err(Failure::Ended)
}
