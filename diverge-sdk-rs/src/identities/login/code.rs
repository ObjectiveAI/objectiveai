//! Signing an identity in by OpenID Connect's Authorization Code flow,
//! with PKCE and a loopback redirect.

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rand::RngCore as _;
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use zeroize::Zeroizing;

use super::device::scoped;
use crate::file_lock;
use crate::identities::claims::claims;
use crate::identities::discovery::{at, discover};
use crate::identities::identities::folder_owned;
use crate::identities::oauth::{Answer, Tokens, post};
use crate::identities::record::{Record, write};
use crate::identities::{Error, Identities, Name};

/// The path the issuer redirects the browser to, on the loopback port.
const CALLBACK: &str = "/callback";

/// The most of a redirect's request read before its first line must
/// have ended.
const REQUEST_LINE: usize = 16 * 1024;

/// An Authorization Code login begun: the URI a person's browser opens
/// to sign in, and the redirect waited for. It holds the identity's lock
/// from [`login_code`](Identities::login_code) on, and the loopback
/// listener the issuer redirects to, and lets both go when it is dropped
/// — at the end of [`wait`](Self::wait), however the login ended, or
/// unwaited.
pub struct Authorizing {
    /// Where the person signs in: the issuer's authorization endpoint,
    /// with the request in its query.
    pub authorization_uri: String,
    identities: Identities,
    name: Name,
    _held: file_lock::Held,
    listener: TcpListener,
    issuer: String,
    client_id: String,
    token_endpoint: String,
    scope: String,
    resource: Option<String>,
    redirect_uri: String,
    state: Zeroizing<String>,
    nonce: Zeroizing<String>,
    verifier: Zeroizing<String>,
}

impl Identities {
    /// Begin signing `name` in by OpenID Connect's Authorization Code
    /// flow, as the public client `client_id`, asking for `scope` —
    /// `openid` and `offline_access` added where it lacks them — and,
    /// where given, for an access token for `resource`. See the
    /// [rules](crate::identities), rule 13. The identity's lock is taken
    /// first, waiting while another holds it, and held by the
    /// [`Authorizing`] until the login ends.
    pub async fn login_code(&self, name: &Name, client_id: &str, scope: &str, resource: Option<&str>) -> Result<Authorizing, Error> {
        let paths = self.paths(name);
        folder_owned(&paths.folder).await?;
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        self.authorize(held, name, client_id, scope, resource).await
    }

    /// [`login_code`](Self::login_code), the lock taken only if nobody
    /// holds it: [`Error::Held`] at once if another login or a reader of
    /// the identity holds it, and nothing asked of the issuer.
    pub async fn try_login_code(&self, name: &Name, client_id: &str, scope: &str, resource: Option<&str>) -> Result<Authorizing, Error> {
        let paths = self.paths(name);
        folder_owned(&paths.folder).await?;
        let Some(held) = file_lock::try_exclusive(&paths.lock).await.map_err(Error::Lock)? else {
            return Err(Error::Held(name.clone()));
        };
        self.authorize(held, name, client_id, scope, resource).await
    }

    /// The login begun, the identity's lock in hand: the document read,
    /// the loopback port bound, the secrets drawn and the authorization
    /// URI made — a failure here lets the lock go at once.
    async fn authorize(&self, held: file_lock::Held, name: &Name, client_id: &str, scope: &str, resource: Option<&str>) -> Result<Authorizing, Error> {
        let issuer = name.issuer.as_str();
        let discovery = discover(&self.http, issuer).await?;
        let lacks = || Error::Discovery(format!("the document at `{}` does not offer the authorization code flow", at(issuer)));
        let endpoint = discovery.authorization_endpoint.clone().ok_or_else(lacks)?;
        if !discovery.response_types_supported.iter().any(|kind| kind == "code") {
            return Err(lacks());
        }
        if let Some(methods) = &discovery.code_challenge_methods_supported
            && !methods.iter().any(|method| method == "S256")
        {
            return Err(Error::Discovery(format!("the document at `{}` does not offer PKCE's S256", at(issuer))));
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.map_err(|error| Error::Callback(error.to_string()))?;
        let port = listener.local_addr().map_err(|error| Error::Callback(error.to_string()))?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK}");
        let state = secret();
        let nonce = secret();
        let verifier = secret();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let scope = scoped(scope);
        let mut uri = reqwest::Url::parse(&endpoint).map_err(|error| Error::Discovery(format!("the authorization endpoint `{endpoint}` is no URL: {error}")))?;
        {
            let mut query = uri.query_pairs_mut();
            query
                .append_pair("response_type", "code")
                .append_pair("client_id", client_id)
                .append_pair("redirect_uri", &redirect_uri)
                .append_pair("scope", &scope)
                .append_pair("state", &state)
                .append_pair("nonce", &nonce)
                .append_pair("code_challenge", &challenge)
                .append_pair("code_challenge_method", "S256")
                .append_pair("login_hint", &name.subject);
            if let Some(resource) = resource {
                query.append_pair("resource", resource);
            }
        }
        Ok(Authorizing {
            authorization_uri: uri.to_string(),
            identities: self.clone(),
            name: name.clone(),
            _held: held,
            listener,
            issuer: discovery.issuer,
            client_id: client_id.to_string(),
            token_endpoint: discovery.token_endpoint,
            scope,
            resource: resource.map(str::to_string),
            redirect_uri,
            state,
            nonce,
            verifier,
        })
    }
}

impl Authorizing {
    /// End the login with no browser: the authorization request sent by
    /// this end itself, the subject and `password` as its Basic
    /// credentials, RFC 7617 — how an agent signs in where the issuer's
    /// authorization endpoint takes them, as Diverge's own does. The
    /// rules:
    ///
    /// 1. **The redirect read, not followed.** A `302` whose `Location`
    ///    is the redirect URI has its query taken as a browser's redirect
    ///    would be — see [`wait`](Self::wait), rules 3 to 5.
    /// 2. **Refused.** A `401` is [`Error::Rejected`]: the issuer refused
    ///    the subject and the password.
    /// 3. **Any other answer** is [`Error::Callback`], with its status
    ///    and its body.
    pub async fn with_password(self, password: &str) -> Result<Name, Error> {
        let credentials = Zeroizing::new(STANDARD.encode(format!("{}:{password}", self.name.subject)));
        let response = self
            .identities
            .http
            .get(&self.authorization_uri)
            .header(reqwest::header::AUTHORIZATION, format!("Basic {}", credentials.as_str()))
            .send()
            .await
            .map_err(Error::Http)?;
        let status = response.status();
        // Rule 2.
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(Error::Rejected);
        }
        // Rule 1.
        if status.is_redirection()
            && let Some(location) = response.headers().get(reqwest::header::LOCATION).and_then(|value| value.to_str().ok())
            && let Some(rest) = location.strip_prefix(self.redirect_uri.as_str())
            && (rest.is_empty() || rest.starts_with('?'))
        {
            let query = rest.strip_prefix('?').unwrap_or_default().to_string();
            return self.redirected(&query).await;
        }
        // Rule 3.
        let body = response.text().await.unwrap_or_default();
        Err(Error::Callback(format!("the authorization endpoint answered {status}: {body}")))
    }

    /// Wait for the issuer's redirect, exchange its code, and write the
    /// identity. The rules:
    ///
    /// 1. **The redirect.** Connections to the loopback port are taken
    ///    until one is `GET /callback?…`; any other is answered `404`
    ///    and the wait goes on. Nothing bounds the wait: it ends when
    ///    the person finishes, or the caller drops it.
    /// 2. **The browser's answer** is plain text, never a page: that the
    ///    sign-in is done, or why it failed.
    /// 3. **The query.** An `error` of `access_denied` is
    ///    [`Error::Denied`], any other `error` [`Error::Refused`]; a
    ///    `state` that is not the one sent is [`Error::State`], and
    ///    nothing is exchanged; no `code` is [`Error::Callback`].
    /// 4. **The exchange.** The code, the redirect URI and the PKCE
    ///    verifier are sent to the token endpoint once.
    /// 5. **The identity.** The answer's ID token must carry the `nonce`
    ///    sent, else [`Error::Nonce`]; it is written as every login's
    ///    answer is, replacing an identity already there, and an answer
    ///    for another subject is [`Error::OtherSubject`], nothing
    ///    written.
    pub async fn wait(self) -> Result<Name, Error> {
        loop {
            let (mut stream, _) = self.listener.accept().await.map_err(|error| Error::Callback(error.to_string()))?;
            // Rule 1.
            let Some(query) = callback(&mut stream).await else {
                let _ = respond(&mut stream, "404 Not Found", "Not found.").await;
                continue;
            };
            let outcome = self.redirected(&query).await;
            // Rule 2.
            let said = match &outcome {
                Ok(_) => "Signed in. This window may be closed.".to_string(),
                Err(error) => format!("The sign-in failed: {error}"),
            };
            let status = if outcome.is_ok() { "200 OK" } else { "400 Bad Request" };
            let _ = respond(&mut stream, status, &said).await;
            return outcome;
        }
    }

    /// Rules 3 to 5, for the redirect's `query`.
    async fn redirected(&self, query: &str) -> Result<Name, Error> {
        let pairs: Vec<(String, String)> = reqwest::Url::parse(&format!("http://127.0.0.1/?{query}"))
            .map_err(|error| Error::Callback(error.to_string()))?
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        let get = |key: &str| pairs.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str());
        // Rule 3.
        if let Some(error) = get("error") {
            if error == "access_denied" {
                return Err(Error::Denied);
            }
            return Err(Error::Refused { code: error.to_string(), description: get("error_description").map(str::to_string) });
        }
        if get("state") != Some(self.state.as_str()) {
            return Err(Error::State);
        }
        let code = get("code").ok_or_else(|| Error::Callback("the redirect carried no code".to_string()))?;
        // Rule 4.
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("client_id", self.client_id.as_str()),
            ("code_verifier", self.verifier.as_str()),
        ];
        let tokens: Tokens = match post::<Tokens>(&self.identities.http, &self.token_endpoint, &form).await? {
            Answer::Answered(tokens) => tokens,
            Answer::Refused(refusal) => return Err(Error::Refused { code: refusal.error, description: refusal.error_description }),
        };
        // Rule 5.
        let nonce = tokens.id_token.as_deref().and_then(claims).and_then(|claims| claims.nonce);
        if nonce.as_deref() != Some(self.nonce.as_str()) {
            return Err(Error::Nonce);
        }
        let record = Record::signed_in(&self.issuer, &self.client_id, &self.token_endpoint, &self.scope, self.resource.as_deref(), tokens)?;
        let answered = record.name();
        if answered != self.name {
            return Err(Error::OtherSubject { expected: self.name.clone(), answered });
        }
        write(&self.identities.paths(&answered), &record).await?;
        Ok(answered)
    }
}

/// 32 random bytes from the OS's generator, as unpadded base64url: 43
/// characters, as a PKCE verifier needs and a state and a nonce may be.
fn secret() -> Zeroizing<String> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    rand::rng().fill_bytes(&mut *bytes);
    Zeroizing::new(URL_SAFE_NO_PAD.encode(*bytes))
}

/// The query of a request that is `GET /callback?…`, read from its
/// first line; `None` for any other request, or one whose first line
/// does not end within [`REQUEST_LINE`] bytes.
async fn callback(stream: &mut TcpStream) -> Option<String> {
    let mut read = Vec::new();
    let mut buffer = [0u8; 1024];
    while !read.windows(2).any(|pair| pair == b"\r\n") {
        if read.len() > REQUEST_LINE {
            return None;
        }
        let count = stream.read(&mut buffer).await.ok()?;
        if count == 0 {
            return None;
        }
        read.extend_from_slice(&buffer[..count]);
    }
    let text = String::from_utf8_lossy(&read);
    let line = text.split("\r\n").next()?;
    let mut parts = line.split(' ');
    let (method, target) = (parts.next()?, parts.next()?);
    if method != "GET" {
        return None;
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    (path == CALLBACK).then(|| query.to_string())
}

/// A plain-text answer to the browser, and the connection closed.
async fn respond(stream: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await
}
