//! Spending the refresh token for new tokens.

use super::Error;
use super::oauth::{Answer, Tokens, post};
use super::record::Record;

/// Refresh `record` at its token endpoint, OAuth 2.0 §6, as its
/// public client: its refresh token, its client id, its `resource`
/// where it has one, and `jwk`, the public half of the identity's key
/// on this host, which the ID token answered is to name — see
/// [`Key`](super::Key). The answer's tokens are taken into
/// `record`; the caller writes it. A refusal as `invalid_grant` is
/// [`Error::SignedOut`], any other [`Error::Refused`].
pub(super) async fn refresh(http: &reqwest::Client, record: &mut Record, jwk: &str) -> Result<(), Error> {
    let mut form = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", record.refresh_token.as_str()),
        ("client_id", record.client_id.as_str()),
        ("jwk", jwk),
    ];
    if let Some(resource) = &record.resource {
        form.push(("resource", resource.as_str()));
    }
    let answer: Answer<Tokens> = post(http, &record.token_endpoint, &form).await?;
    match answer {
        Answer::Answered(tokens) => record.take(tokens),
        Answer::Refused(refusal) if refusal.error == "invalid_grant" => Err(Error::SignedOut(record.name())),
        Answer::Refused(refusal) => Err(Error::Refused { code: refusal.error, description: refusal.error_description }),
    }
}
