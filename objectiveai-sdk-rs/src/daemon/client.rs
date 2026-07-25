//! THE daemon client: one [`Client`] holding the daemon's base
//! address + auth signature + a shared HTTP pool, minting every
//! per-endpoint structure — the materialized SSE listeners, the
//! laboratory file tree, the viewer-plugin bundle download — and
//! implementing [`CommandExecutor`] itself for `POST /execute`
//! (feature `cli-executor`; no separate executor struct).
//!
//! Listener methods are async and return the materialized structure
//! only once its SSE stream has actually OPENED (the `Open` frame is
//! awaited — a bad address or a 401 is an `Err` here, never a
//! silently-frozen view). One listener = one connection; reconnection
//! is the caller's loop.

use futures::StreamExt;
use reqwest_eventsource::RequestBuilderExt;

use super::Error;

/// The daemon's HTTP surface behind one handle — see the module docs.
/// Construct with [`Client::new`] from the daemon's published base
/// address (e.g. `http://127.0.0.1:49152`).
#[derive(Clone)]
pub struct Client {
    /// The daemon's base address, trailing-slash-trimmed; routes are
    /// appended.
    address: String,
    /// Optional auth signature, sent as the `X-OBJECTIVEAI-SIGNATURE`
    /// header on every request.
    signature: Option<String>,
    /// The client's own caller [`Identity`](crate::identity::Identity)
    /// — the DEFAULT for surfaces that carry one (`/execute` headers):
    /// a per-call identity, when given, wins over it.
    identity: Option<crate::identity::Identity>,
    /// The shared connection pool — every minted structure rides it.
    http: reqwest::Client,
}

impl Client {
    pub fn new(address: impl Into<String>) -> Self {
        let address = address.into();
        Self {
            address: address.trim_end_matches('/').to_string(),
            signature: None,
            identity: None,
            http: reqwest::Client::new(),
        }
    }

    /// Attach the daemon auth signature (the pre-derived
    /// `sha256=<hex(SHA256(DAEMON_SECRET))>`). Without it the daemon
    /// must be running without a secret.
    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Attach this client's caller identity — the default identity
    /// bag for every surface that carries one (`/execute`'s
    /// `X-OBJECTIVEAI-*` headers). A per-call identity passed to
    /// [`execute`](crate::cli::command::CommandExecutor::execute)
    /// overrides it call-by-call.
    pub fn identity(mut self, identity: crate::identity::Identity) -> Self {
        self.identity = Some(identity);
        self
    }

    /// The daemon's base address (trimmed), as constructed.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Open `{address}{route}` as an SSE stream and WAIT for it to
    /// open — the `Open` frame (the 2xx response) must arrive before
    /// this returns. 401s and transport failures surface here.
    pub(crate) async fn open_sse(
        &self,
        route: &str,
    ) -> Result<reqwest_eventsource::EventSource, Error> {
        let mut request = self
            .http
            .get(format!("{}{route}", self.address))
            .header("Accept", "text/event-stream");
        if let Some(signature) = &self.signature {
            request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        let mut source = request.eventsource()?;
        // reqwest-eventsource yields `Open` first on a successful
        // response; an error (invalid status included) or immediate
        // end means the stream never opened.
        match source.next().await {
            Some(Ok(reqwest_eventsource::Event::Open)) => Ok(source),
            Some(Err(e)) => Err(Error::Open(e)),
            Some(Ok(reqwest_eventsource::Event::Message(_))) | None => {
                Err(Error::Closed)
            }
        }
    }

    /// A plain request to `{address}{route}` with the signature
    /// stamped.
    pub(crate) fn request(
        &self,
        method: reqwest::Method,
        route: &str,
    ) -> reqwest::RequestBuilder {
        let mut request = self
            .http
            .request(method, format!("{}{route}", self.address));
        if let Some(signature) = &self.signature {
            request = request.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        request
    }

    /// The `/listen` broadcast as a typed stream —
    /// [`CommandListener`](super::command_listener::CommandListener):
    /// every CLI execution the daemon runs, request + response items,
    /// dispatched onto the command tree's leaf types.
    #[cfg(feature = "cli")]
    pub async fn command_listener(
        &self,
    ) -> Result<super::command_listener::CommandListener, Error> {
        super::command_listener::CommandListener::connect(self).await
    }

    /// The `/agents/instances/list` view: every agent's
    /// active/inactive status, live.
    pub async fn agents_instances_list_listener(
        &self,
    ) -> Result<super::agents_instances_list_listener::AgentsInstancesListListener, Error>
    {
        super::agents_instances_list_listener::AgentsInstancesListListener::connect(
            self,
        )
        .await
    }

    /// The `/agents/instances/{aih}` view: ONE agent's full
    /// conversation, DB history + live rows.
    pub async fn agents_instances_listener(
        &self,
        agent_instance_hierarchy: &str,
    ) -> Result<super::agents_instances_listener::AgentsInstancesListener, Error>
    {
        super::agents_instances_listener::AgentsInstancesListener::connect(
            self,
            agent_instance_hierarchy,
        )
        .await
    }

    /// The `/laboratories/list` view: the live laboratories merge
    /// (connected ∪ local scan).
    pub async fn laboratories_list_listener(
        &self,
    ) -> Result<super::laboratories_list_listener::LaboratoriesListListener, Error>
    {
        super::laboratories_list_listener::LaboratoriesListListener::connect(self)
            .await
    }

    /// The `/laboratories/{id}` view: ONE laboratory's record with
    /// attachments.
    pub async fn laboratories_listener(
        &self,
        laboratory_id: &str,
    ) -> Result<super::laboratories_listener::LaboratoriesListener, Error> {
        super::laboratories_listener::LaboratoriesListener::connect(
            self,
            laboratory_id,
        )
        .await
    }

    /// The `/channels` offer-lifecycle view — answer its offers with
    /// [`accept_channel`](Self::accept_channel).
    pub async fn channel_listener(
        &self,
    ) -> Result<super::channel_listener::ChannelListener, Error> {
        super::channel_listener::ChannelListener::connect(self).await
    }

    /// Accept an open channel offer: a bare
    /// `POST /channels/{id}/accept` (first-wins). Returns the owner
    /// secret (`S_owner`) from the response body — the per-channel
    /// capability for `channels logs reply|list|open|subscribe` and
    /// `channels close`.
    pub async fn accept_channel(&self, channel_id: &str) -> Result<String, Error> {
        let response = self
            .request(
                reqwest::Method::POST,
                &format!("/channels/{channel_id}/accept"),
            )
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(match status {
                reqwest::StatusCode::NOT_FOUND => Error::NotFound,
                reqwest::StatusCode::CONFLICT => Error::AlreadyAccepted,
                reqwest::StatusCode::UNAUTHORIZED => Error::Unauthorized,
                _ => Error::Status {
                    status,
                    body: response.text().await.unwrap_or_default().trim().to_string(),
                },
            });
        }
        let body = response.text().await?;
        let accepted: super::channel_listener::ChannelAccepted =
            serde_json::from_str(&body)?;
        Ok(accepted.secret)
    }

    /// The `/laboratories/{id}/filetree` view: one laboratory's live
    /// file tree.
    pub async fn file_tree(
        &self,
        laboratory_id: &str,
    ) -> Result<super::file_tree::FileTree, Error> {
        super::file_tree::FileTree::connect_daemon(self, laboratory_id).await
    }

    /// Download a plugin's viewer-extension bundle:
    /// `GET /plugins/{owner}/{name}/{version}/viewer`, the daemon
    /// building it on demand and streaming tar.gz back. LOW-LEVEL by
    /// design — raw bytes, no filesystem; the caller un-tars its own
    /// way. `version` is the plugin repo's v-prefixed git tag
    /// (`v1.2.3`), byte-for-byte.
    pub async fn get_viewer_plugin(
        &self,
        owner: &str,
        name: &str,
        version: &str,
    ) -> Result<super::ViewerPlugin, Error> {
        let response = self
            .request(
                reqwest::Method::GET,
                &format!("/plugins/{owner}/{name}/{version}/viewer"),
            )
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(match status {
                reqwest::StatusCode::NOT_FOUND => Error::NotFound,
                reqwest::StatusCode::UNAUTHORIZED => Error::Unauthorized,
                _ => Error::Status {
                    status,
                    body: response.text().await.unwrap_or_default().trim().to_string(),
                },
            });
        }
        let commit_sha = response
            .headers()
            .get("x-objectiveai-sha")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        Ok(super::ViewerPlugin::new(commit_sha, response))
    }
}

/// The first client→daemon frame on the `/laboratory` host-channel
/// WebSocket (the daemon's ONE remaining WebSocket) — the auth preamble,
/// always sent, even to a secretless daemon (`{"signature": null}`). A
/// daemon holding a `DAEMON_SECRET` verifies the signature and closes
/// the connection on a missing/invalid one; a secretless daemon consumes
/// the envelope and ignores the value. (The HTTP routes authenticate by
/// the `X-OBJECTIVEAI-SIGNATURE` header instead.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "daemon.AuthEnvelope")]
pub struct AuthEnvelope {
    /// The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, or
    /// `null` when the client has none.
    pub signature: Option<String>,
}

/// A transport error's FULL cause chain — `Display` alone hides the
/// interesting part (reqwest's "error decoding response body" says
/// nothing without the hyper cause underneath, e.g. "connection reset"
/// vs "connection closed before message completed").
#[cfg(feature = "cli-executor")]
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// Per-event untagged decode. `Err` is listed first so serde tries it
/// before `Ok` — `cli::Error`'s `type:"error"` constant short-circuits
/// every non-error wire shape, then `Ok(T)` is the fallthrough.
#[cfg(feature = "cli-executor")]
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Line<T> {
    Err(crate::cli::Error),
    Ok(T),
}

#[cfg(feature = "cli-executor")]
impl<T> From<Line<T>> for Result<T, super::ExecuteError> {
    fn from(line: Line<T>) -> Self {
        match line {
            Line::Err(e) => Err(super::ExecuteError::Cli(e)),
            Line::Ok(t) => Ok(t),
        }
    }
}

/// `POST /execute` — the [`Client`] IS the executor: each command is
/// one POST whose result streams back as SSE, run in-process by the
/// daemon with the [`crate::identity::Identity`] headers applied as a
/// per-request config override (a missing header DELETES that field
/// for the run — the daemon never inherits its own resident value).
/// The `plugin_*` trio is deliberately never stamped: plugin caller
/// identity is unspoofable — only the daemon's own `plugins run` may
/// assert it. Dropping the returned stream aborts the request, which
/// cancels the in-process run.
#[cfg(feature = "cli-executor")]
impl crate::cli::command::CommandExecutor for Client {
    type Error = super::ExecuteError;
    type Stream<T>
        = std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<T, super::ExecuteError>> + Send>,
    >
    where
        T: Send + 'static;

    async fn execute<R, T>(
        &self,
        request: R,
        identity: Option<&crate::identity::Identity>,
    ) -> Result<Self::Stream<T>, super::ExecuteError>
    where
        R: crate::cli::command::CommandRequest + Send + serde::Serialize,
        T: crate::cli::command::CommandResponse
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Send
            + 'static,
    {
        use eventsource_stream::Eventsource;
        let mut req = self
            .http
            .post(format!("{}/execute", self.address))
            .header("Accept", "text/event-stream")
            .json(&request);
        if let Some(signature) = &self.signature {
            req = req.header("X-OBJECTIVEAI-SIGNATURE", signature);
        }
        // Per-call identity wins; the client's own is the default.
        if let Some(args) = identity.or(self.identity.as_ref()) {
            if let Some(v) = &args.agent_instance_hierarchy {
                req = req.header("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY", v);
            }
            if let Some(v) = &args.agent_id {
                req = req.header("X-OBJECTIVEAI-AGENT-ID", v);
            }
            if let Some(v) = &args.agent_full_id {
                req = req.header("X-OBJECTIVEAI-AGENT-FULL-ID", v);
            }
            if let Some(v) = &args.agent_remote {
                req = req.header("X-OBJECTIVEAI-AGENT-REMOTE", v);
            }
            if let Some(v) = &args.response_id {
                req = req.header("X-OBJECTIVEAI-RESPONSE-ID", v);
            }
            if let Some(v) = &args.response_ids {
                req = req.header("X-OBJECTIVEAI-RESPONSE-IDS", v);
            }
        }
        let response = req.send().await.map_err(super::ExecuteError::Connect)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(super::ExecuteError::Http(
                status,
                body.trim().to_string(),
            ));
        }

        // The SSE body owns the connection; dropping the returned
        // stream drops the body, which aborts the request and cancels
        // the daemon-side run. `eventsource-stream` parses without
        // reconnecting (the command stream is finite — body end = done).
        let events = response.bytes_stream().eventsource();
        let stream = events.map(|event| match event {
            Ok(event) => match serde_json::from_str::<Line<T>>(&event.data) {
                Ok(line) => line.into(),
                Err(e) => Err(super::ExecuteError::Json(e)),
            },
            // `EventStreamError::Transport` does NOT expose its inner
            // reqwest error via `source()` — unwrap it by hand so the
            // hyper cause chain (reset vs premature close vs framing)
            // actually reaches the message.
            Err(eventsource_stream::EventStreamError::Transport(e)) => Err(
                super::ExecuteError::Sse(format!("transport: {}", error_chain(&e))),
            ),
            Err(e) => Err(super::ExecuteError::Sse(error_chain(&e))),
        });

        Ok(Box::pin(stream))
    }

    async fn execute_one<R, T>(
        &self,
        request: R,
        identity: Option<&crate::identity::Identity>,
    ) -> Result<T, super::ExecuteError>
    where
        R: crate::cli::command::CommandRequest + Send + serde::Serialize,
        T: crate::cli::command::CommandResponse
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Send
            + 'static,
    {
        let mut stream = self.execute::<R, T>(request, identity).await?;
        stream.next().await.ok_or(super::ExecuteError::Empty)?
    }
}
