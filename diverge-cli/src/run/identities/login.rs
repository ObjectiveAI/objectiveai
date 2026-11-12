//! `diverge run identities login`: signing an identity in on the
//! daemon's host, as
//! [`login`](diverge_sdk::daemon::endpoints::identities::login) states
//! it — the code printed, and the wait for its approval left behind.

use clap::Args;
use diverge_sdk::daemon::endpoints::identities::login::{client::execute, client::request, server::response};
use futures_util::StreamExt as _;

use crate::detach;
use crate::ours;
use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Signing an identity in on the daemon's host.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it. Absent, the flags alone.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The issuer, exactly as its discovery document names it: in place
    /// of the body's `issuer`.
    #[arg(long)]
    pub issuer: Option<String>,
    /// The subject to sign in as, the `sub` the approval must name: in
    /// place of the body's `subject`.
    #[arg(long)]
    pub subject: Option<String>,
    /// The public client the daemon signs in as at the issuer: in place
    /// of the body's `client_id`.
    #[arg(long)]
    pub client_id: Option<String>,
    /// The scope asked for, space-separated: in place of the body's
    /// `scope`.
    #[arg(long)]
    pub scope: Option<String>,
    /// What the access token is for, RFC 8707's `resource`: in place of
    /// the body's `resource`.
    #[arg(long)]
    pub resource: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The rules:
///
/// 1. **The request.** The body, every flag given in place of its
///    member; with no body, the flags alone, of which `--issuer`,
///    `--subject` and `--client-id` are needed.
/// 0. **At `identity.diverge.network`** the login is its own way —
///    see [`ours`]: the subject's owner is an account signed in on
///    THIS host, the code the daemon answers is approved by this
///    command as that account, and the login is waited for. Nothing
///    is left behind, `pending` is not printed — there is nothing for
///    anybody to do with it — and every other answer is: `signed_in`,
///    or why not. The rules below, from 2 on, are every other
///    issuer's.
/// 2. **Left behind** — see [`detach`]: this command is the front,
///    and the request is its waiter's. Asked for a schema, there is no
///    request and no waiter.
/// 3. **Printed up to the code.** Every answer is printed, one JSON
///    line, as it comes, up to and with `pending` — the user code and
///    where it is approved. There the waiter leaves, the command ends,
///    and nothing after is printed. No browser is opened. A login
///    refused before it has a code — `held`, `forbidden`, `error` — is
///    printed, and the command ends with it.
/// 4. **Waited to the end, apart.** The waiter reads the answers to the
///    daemon's finish, holding the connection the login lives on: a
///    login always ends, approved, denied, as another identity, or at
///    the code's expiry. Ending the waiter ends the connection, which
///    ends the login on the daemon, and nothing is written.
/// 5. **The outcome is not printed.** A login signed in is an identity
///    on the daemon's host, which `run identities list` shows; one
///    that was not is none.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run identities login", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run identities login", emitter).await;
        }
        None => {}
    }
    // Rule 1.
    let mut request: serde_json::Map<String, serde_json::Value> = match arguments.body.as_deref() {
        Some(_) => body(arguments.body.as_deref())?,
        None => serde_json::Map::new(),
    };
    for (member, value) in [
        ("issuer", arguments.issuer),
        ("subject", arguments.subject),
        ("client_id", arguments.client_id),
        ("scope", arguments.scope),
        ("resource", arguments.resource),
    ] {
        if let Some(value) = value {
            request.insert(member.to_string(), serde_json::Value::String(value));
        }
    }
    let request: request::Frame = body(Some(&serde_json::Value::Object(request).to_string()))?;
    // Rule 0.
    if let Some(ours) = ours::ours(target.root, &request.issuer, &request.subject).await.map_err(Error::Ours)? {
        let handle = target.dial().await?;
        let (mut answers, _cancel) = execute::execute(&handle, &request).await.map_err(shape::exchange::open)?;
        while let Some(item) = answers.next().await {
            match item.map_err(shape::exchange::stream)? {
                response::Frame::Pending(pending) => ours.approve(&pending.user_code).await.map_err(Error::Ours)?,
                answer => {
                    emitter.emit(&answer).await?;
                }
            }
        }
        return Ok(());
    }
    // Rule 2.
    if !detach::waiter() {
        return detach::relay().await.map_err(Error::Detach);
    }
    let handle = target.dial().await?;
    let (mut answers, _cancel) = execute::execute(&handle, &request).await.map_err(shape::exchange::open)?;
    // Rules 3 to 5.
    let mut left = false;
    while let Some(item) = answers.next().await {
        let answer = item.map_err(shape::exchange::stream)?;
        if left {
            continue;
        }
        emitter.emit(&answer).await?;
        if matches!(answer, response::Frame::Pending(_)) {
            detach::leave().await;
            left = true;
        }
    }
    Ok(())
}
