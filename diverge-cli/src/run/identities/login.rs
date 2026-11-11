//! `diverge run identities login`: signing an identity in on the
//! daemon's host, as
//! [`login`](diverge_sdk::daemon::endpoints::identities::login) states
//! it — the code approved in a browser here, or anywhere.

use clap::Args;
use diverge_sdk::daemon::endpoints::identities::login::{client::execute, client::request, server::response};
use futures_util::StreamExt as _;
use tokio::io::AsyncReadExt as _;

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
    /// Open no browser at the code: for a client with none, whose
    /// person approves it on another device.
    #[arg(long)]
    pub no_browser: bool,
    /// Sign in by the identity's password, read from stdin to its end,
    /// the trailing line end dropped, in place of the body's
    /// `password`: no code, no browser.
    #[arg(long)]
    pub password_stdin: bool,
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
/// 2. **Every answer printed**, one JSON line each, as it comes.
/// 3. **The browser.** At `pending`, the browser here is opened at the
///    page with the code in it where the issuer gives one, else at the
///    verification page — unless `--no-browser`. A browser that will not
///    open is no error: the line printed names the page, and any device
///    may open it. A password login sends no `pending`, so no browser
///    opens. There is no Authorization Code login here: its redirect
///    comes to a loopback port on the host the browser runs on, which
///    the daemon's host need not be — `diverge identities login` signs
///    in that way on this host.
/// 5. **By password.** `--password-stdin` reads the password from stdin
///    and sends it as the body's `password`: the daemon signs in by the
///    Authorization Code flow with no browser, on its own host — how an
///    agent signs in at Diverge's own issuer.
/// 4. **To the end.** The answers are read to the daemon's finish: a
///    login always ends, approved, denied, as another identity, or at
///    the code's expiry. Ending the command ends the connection, which
///    ends the login on the daemon, and nothing is written.
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
        ("password", if arguments.password_stdin { Some(read_password().await?) } else { None }),
    ] {
        if let Some(value) = value {
            request.insert(member.to_string(), serde_json::Value::String(value));
        }
    }
    let request: request::Frame = body(Some(&serde_json::Value::Object(request).to_string()))?;
    let handle = target.dial().await?;
    let (mut answers, _cancel) = execute::execute(&handle, &request).await.map_err(shape::exchange::open)?;
    // Rules 2 to 4.
    while let Some(item) = answers.next().await {
        let answer = item.map_err(shape::exchange::stream)?;
        emitter.emit(&answer).await?;
        if let response::Frame::Pending(pending) = &answer
            && !arguments.no_browser
        {
            let page = pending.verification_uri_complete.as_deref().unwrap_or(&pending.verification_uri);
            let _ = webbrowser::open(page);
        }
    }
    Ok(())
}

/// The password on stdin, all of it, the trailing line end dropped. An
/// empty one is refused.
async fn read_password() -> Result<String, Error> {
    let mut text = String::new();
    tokio::io::stdin().read_to_string(&mut text).await.map_err(Error::Stdin)?;
    let trimmed = text.trim_end_matches(['\r', '\n']).len();
    text.truncate(trimmed);
    if text.is_empty() {
        return Err(Error::EmptyPassword);
    }
    Ok(text)
}
