//! An ObjectiveAI MCP plugin, in as few moving parts as one can have.
//!
//! Everything specific to running inside ObjectiveAI — the transport,
//! the port binding, the `initialize` reply, the command extension —
//! is [`objectiveai_mcp_plugin_framework`]'s job. What is left here is
//! the part that is actually yours: the tools, and what they do.
//!
//! `rename.sh` handles `NAME`, the package and the binary. What is
//! left for you is `PORT` — which must match `mcp.port` in
//! `objectiveai.json`, since the host publishes the port the manifest
//! names — and the tools.

use std::convert::Infallible;
use std::sync::Arc;

// Brings `rmcp` into scope under the name the `#[tool_router]` and
// `#[tool]` macros expand to. Depending on `rmcp` separately would
// risk two versions in one binary, where a `ToolRouter` built by the
// macros would not fit `serve`.
use objectiveai_mcp_plugin_framework::rmcp;
// Likewise the SDK, whose `CommandExecutor` trait every `execute` is
// generic over: a separately-resolved copy would be a different trait.
use objectiveai_mcp_plugin_framework::objectiveai_sdk;
use futures::StreamExt;
use objectiveai_mcp_plugin_framework::tools::Tools;
use objectiveai_mcp_plugin_framework::{db, sqlx};
use objectiveai_sdk::cli::command::RequestBase;
use objectiveai_sdk::cli::command::agents::instances::get;
use objectiveai_sdk::cli::command::channels;
use rmcp::handler::server::tool::ToolRoute;
use rmcp::handler::server::wrapper::{Json, Parameters};
use sqlx::Row as _;

/// Must match `mcp.port` in `objectiveai.json`.
const PORT: u16 = 8080;
/// The routing prefix ObjectiveAI derives — see the module docs.
const NAME: &str = "objectiveai-plugin-scaffold";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The argument that gates the pair below. Declared by the AGENT, in
/// its plugin entry — not by this process, and not changeable while it
/// runs.
///
/// Read strictly as a boolean: JSON `true` and nothing else. Absent,
/// `null`, `0`, `"true"` — all off. Argument values are free-form JSON
/// that some human typed into an agent definition, so the only safe
/// reading of "is this feature on" is the exact one.
const SWITCH_ARGUMENT: &str = "switch";

const SWITCH_TOOL: &str = "scaffold_switch_deleteme";
const SWITCHED_TOOL: &str = "scaffold_switched_deleteme";

/// Created on first use rather than by a migration, because a plugin
/// container is ephemeral and there is nowhere to run one.
///
/// The database is the DAEMON's, tunnelled in — not a private one — so
/// a plugin shares it with ObjectiveAI's own tables and with every
/// other plugin. Two habits follow, and both are in this statement: own
/// a distinctly named table rather than writing into someone else's,
/// and scope rows by the agent they belong to, since the next container
/// over is a different agent looking at the same rows.
const CREATE_NOTES: &str = "
    CREATE TABLE IF NOT EXISTS scaffold_notes_deleteme (
        agent_instance_hierarchy TEXT        NOT NULL,
        note_key                 TEXT        NOT NULL,
        note_value               TEXT        NOT NULL,
        written_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
        PRIMARY KEY (agent_instance_hierarchy, note_key)
    )
";

/// Runs [`CREATE_NOTES`] once per process, however many tools race to
/// use it — the same shape [`db::connect`] uses, and for the same
/// reason: the work is idempotent but the round trip is not free.
static NOTES_TABLE: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// Where a credential obtained through a viewer channel is kept, so it
/// is asked for once rather than once per call.
const CREATE_CREDENTIALS: &str = "
    CREATE TABLE IF NOT EXISTS scaffold_credentials_deleteme (
        agent_instance_hierarchy TEXT        NOT NULL PRIMARY KEY,
        credential               TEXT        NOT NULL,
        obtained_at              TIMESTAMPTZ NOT NULL DEFAULT now()
    )
";

static CREDENTIALS_TABLE: tokio::sync::OnceCell<()> =
    tokio::sync::OnceCell::const_new();

/// The channel's discriminator. A user surface decides from this
/// whether an offer is one it knows how to answer, so it wants to name
/// the EXCHANGE, not the plugin.
const CREDENTIAL_CHANNEL_KEY: &str = "scaffold.credential";

/// How long to wait for a viewer to accept the offer. `channels
/// publish` blocks until one does, and is UNCAPPED without this — a
/// plugin that omits it waits for a human forever.
const ACCEPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// How long to wait, after acceptance, for the reply carrying the
/// credential. Separate from [`ACCEPT_TIMEOUT`] because they are
/// different waits: one is "is anyone there", the other is "has the
/// person finished typing".
///
/// Enforced by the daemon, not here: `timeout_seconds` becomes a
/// whole-stream deadline over every command, anchored at first poll,
/// which yields a `Timeout` error item and ends the stream.
const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Which tools to actually serve.
///
/// `Plugin::tool_router()` declares every tool this plugin could ever
/// have; this decides which of them exist right now. Two independent
/// gates, and they are different in kind:
///
/// - the ARGUMENT gate is fixed for the process's life, because the
///   host stamps the arguments at container create and nothing
///   rewrites them. A plugin the agent did not ask to have a switch
///   never serves one, and no call can change that.
/// - the SWITCH gate moves at runtime, which is the whole point of
///   [`Tools::replace`].
///
/// Filtering a full router by name, rather than assembling routes by
/// hand, means the macros stay the single declaration of what a tool
/// IS — this only decides whether it is currently served.
fn served_routes(switched_on: bool) -> Vec<ToolRoute<Plugin>> {
    // `as_bool` is `Some` only for a JSON boolean, so every other
    // shape — missing, `null`, a number, the STRING "true" — falls
    // through to off. Deliberately not lenient: a plugin that guesses
    // what someone meant by `"true"` is a plugin that will one day
    // guess wrong about a feature that should have stayed off.
    let has_switch = objectiveai_mcp_plugin_framework::arguments()
        .get(SWITCH_ARGUMENT)
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    Plugin::tool_router()
        .into_iter()
        .filter(|route| {
            let name = route.attr.name.as_ref();
            if name == SWITCH_TOOL {
                has_switch
            } else if name == SWITCHED_TOOL {
                has_switch && switched_on
            } else {
                true
            }
        })
        .collect()
}

/// The pool, with `ddl` guaranteed to have run once.
///
/// Note the queries are `sqlx::query`, not the `sqlx::query!` MACRO.
/// The macro checks SQL against a live database AT COMPILE TIME, which
/// would make this plugin unbuildable without one — and the database a
/// plugin talks to does not exist until a host creates its container.
async fn table_pool(
    table: &'static tokio::sync::OnceCell<()>,
    ddl: &'static str,
) -> Result<db::Pool, String> {
    let pool = db::connect(Default::default())
        .await
        .map_err(|error| error_chain("connect to the database", &*error))?;

    table
        .get_or_try_init(|| async { sqlx::query(ddl).execute(&pool).await.map(|_| ()) })
        .await
        .map_err(|error| error_chain("create the table", &error))?;

    Ok(pool)
}

/// `source` chains are where the actual cause lives — sqlx's top-level
/// `Display` is often just "error returned from database server", and
/// the SDK's executor errors are the same shape. Whoever reads this
/// gets one string, so it has to be the whole story.
fn error_chain(doing: &str, error: &dyn std::error::Error) -> String {
    let mut message = format!("{doing}: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    message
}

/// The same, as the protocol-level error the note tools return.
async fn notes_pool() -> Result<db::Pool, rmcp::ErrorData> {
    table_pool(&NOTES_TABLE, CREATE_NOTES)
        .await
        .map_err(|message| rmcp::ErrorData::internal_error(message, None))
}

/// A [`RequestBase`] carrying nothing but a wall-clock cap.
fn capped(timeout: std::time::Duration) -> RequestBase {
    RequestBase {
        // Whole seconds, and never zero — zero is rejected at parse
        // time, and rounding a sub-second remainder down to it would
        // turn "almost out of time" into "invalid request".
        timeout_seconds: Some(timeout.as_secs().max(1)),
        ..Default::default()
    }
}

/// Ask a user surface for a credential, over one channel, and close it.
///
/// The shape of the exchange, which is not obvious from the commands:
///
/// 1. `channels publish` offers the channel and BLOCKS until a
///    `/channels` client accepts, returning the id and `S_pub`.
/// 2. `channels logs subscribe` waits for the owner's reply. One call
///    suffices: a publisher's reads are scoped to `reply` entries, so
///    the offer never comes back as its own answer.
/// 3. Entries are ENVELOPES. The content lives behind `channels logs
///    open`, which is why finding the reply and reading it are two
///    round trips.
/// 4. The channel is closed either way. A channel left open is a user
///    surface left waiting on a plugin that has stopped caring.
///
/// Both waits are bounded by the base `timeout_seconds`, which the
/// daemon enforces for every command — no clock is kept here.
async fn request_credential(url: &str) -> Result<String, CredentialFailure> {
    let executor = objectiveai_mcp_plugin_framework::command_executor();

    let offer = channels::publish::execute(
        &executor,
        channels::publish::Request {
            path_type: channels::publish::Path::ChannelsPublish,
            key: CREDENTIAL_CHANNEL_KEY.to_string(),
            // Opaque to the daemon — this is for whoever accepts, and
            // naming the endpoint is the whole basis on which a person
            // decides whether to hand over a credential.
            details: serde_json::json!({ "url": url }),
            message: format!("A plugin is asking for a credential for {url}."),
            base: capped(ACCEPT_TIMEOUT),
        },
        None,
    )
    .await
    .map_err(|error| failed("publish", error_chain("publish the channel", &error)))?;

    let credential = collect_credential(&executor, &offer).await;

    // Close on both paths. `S_pub` authorizes it, and a failure to
    // close is not worth overwriting the real outcome with.
    let _ = channels::close::execute(
        &executor,
        channels::close::Request {
            path_type: channels::close::Path::ChannelsClose,
            channel_id: offer.channel_id.clone(),
            secret: offer.secret.clone(),
            base: Default::default(),
        },
        None,
    )
    .await;

    credential
}

/// Wait for the owner's reply on an accepted channel and read the
/// credential out of it.
async fn collect_credential(
    executor: &objectiveai_mcp_plugin_framework::command_executor::Executor,
    offer: &channels::publish::Response,
) -> Result<String, CredentialFailure> {
    // ONE subscribe is enough, for a reason worth knowing: the daemon
    // scopes each role to a direction, and a PUBLISHER reads only
    // `reply` entries. The offer this plugin just published is an
    // owner-side entry, so it is never handed back as if it were the
    // answer — there is nothing to skip past and no cursor to carry.
    //
    // The timeout is the base cap, which the daemon applies to every
    // command as a whole-stream deadline anchored at first poll. It
    // arrives as a `Timeout` error item, so the `Err` arm below reports
    // it; nothing here needs its own clock.
    let entries = channels::logs::subscribe::execute(
        executor,
        channels::logs::subscribe::Request {
            path_type: channels::logs::subscribe::Path::ChannelsLogsSubscribe,
            channel_id: offer.channel_id.clone(),
            secret: offer.secret.clone(),
            after_id: None,
            limit: None,
            base: capped(REPLY_TIMEOUT),
        },
        None,
    )
    .await
    .map_err(|error| failed("subscribe", error_chain("subscribe", &error)))?;

    let mut entries = std::pin::pin!(entries);
    let reply_id = loop {
        match entries.next().await {
            Some(Ok(channels::logs::subscribe::ResponseItem::Item(
                channels::logs::list::ChannelLogEntry::Reply { details_id, .. },
            ))) => break details_id,
            // Unreachable while a publisher reads only replies. Skipped
            // rather than rejected so that widening the role's
            // directions could never turn this into a hard failure.
            Some(Ok(channels::logs::subscribe::ResponseItem::Item(_))) => continue,
            Some(Ok(channels::logs::subscribe::ResponseItem::ChannelClosed(_))) => {
                return Err(failed(
                    "subscribe",
                    "the channel closed before a reply arrived",
                ));
            }
            Some(Err(error)) => {
                return Err(failed("subscribe", error_chain("subscribe", &error)));
            }
            None => {
                return Err(failed(
                    "subscribe",
                    "the subscription ended before a reply arrived",
                ));
            }
        }
    };

    let opened = channels::logs::open::execute(
        executor,
        channels::logs::open::Request {
            path_type: channels::logs::open::Path::ChannelsLogsOpen,
            channel_id: offer.channel_id.clone(),
            secret: offer.secret.clone(),
            entry_id: reply_id,
            base: Default::default(),
        },
        None,
    )
    .await
    .map_err(|error| failed("open", error_chain("open the reply", &error)))?;

    let content = match opened {
        channels::logs::open::Response::Entry { content, .. } => content,
        channels::logs::open::Response::NotFound => {
            return Err(failed("open", "the reply entry was gone"));
        }
    };

    content
        .get("credential")
        .and_then(|value| value.as_str())
        .filter(|credential| !credential.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            failed(
                "credential",
                "the reply carried no non-empty string `credential` field",
            )
        })
}

/// Look up, or obtain and store, then call.
async fn credential_call(url: &str) -> Result<CredentialCall, CredentialFailure> {
    // A failed lookup is FATAL, and deliberately so. "The read
    // errored" and "there is no stored credential" are the same
    // silence from here, and guessing the second would bother a human
    // for a credential that already exists — then overwrite the good
    // one with whatever came back. Refusing costs a retry; guessing
    // costs the stored credential.
    let pool = table_pool(&CREDENTIALS_TABLE, CREATE_CREDENTIALS)
        .await
        .map_err(|message| failed("database", message))?;

    let stored: Option<String> = sqlx::query(
        "SELECT credential FROM scaffold_credentials_deleteme
         WHERE agent_instance_hierarchy = $1",
    )
    .bind(notes_scope())
    .fetch_optional(&pool)
    .await
    .map_err(|error| failed("database", error_chain("read the credential", &error)))?
    .map(|row| row.get("credential"));

    let (credential, credential_source) = match stored {
        Some(credential) => (credential, "database"),
        None => return credential_from_channel(url, pool).await,
    };

    let response = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::AUTHORIZATION, &credential)
        .send()
        .await
        .map_err(|error| failed("request", error_chain("call the endpoint", &error)))?;

    // The STATUS is reported, not enforced. A 401 is a real answer to
    // "call this with my credential", and turning it into an error
    // would hide the one result that says the credential is stale.
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|error| failed("request", error_chain("read the response", &error)))?;

    Ok(CredentialCall {
        credential_source: credential_source.to_string(),
        status,
        body,
    })
}

/// Ask a human for the credential through the viewer channel, store
/// it, and make the call. Reached only when the lookup SUCCEEDED and
/// found nothing — see `credential_call`.
async fn credential_from_channel(
    url: &str,
    pool: db::Pool,
) -> Result<CredentialCall, CredentialFailure> {
    let credential = request_credential(url).await?;

    // Stored only once it is in hand. Writing before the exchange
    // completed would leave a half-credential behind for the next
    // call to trust.
    sqlx::query(
        "INSERT INTO scaffold_credentials_deleteme
             (agent_instance_hierarchy, credential)
         VALUES ($1, $2)
         ON CONFLICT (agent_instance_hierarchy) DO UPDATE
             SET credential = EXCLUDED.credential, obtained_at = now()",
    )
    .bind(notes_scope())
    .bind(&credential)
    .execute(&pool)
    .await
    .map_err(|error| failed("store", error_chain("store the credential", &error)))?;

    let response = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::AUTHORIZATION, &credential)
        .send()
        .await
        .map_err(|error| failed("request", error_chain("call the endpoint", &error)))?;

    // The STATUS is reported, not enforced — see `credential_call`.
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|error| failed("request", error_chain("read the response", &error)))?;

    Ok(CredentialCall {
        credential_source: "channel".to_string(),
        status,
        body,
    })
}

/// Which agent's notes these are. Rows are scoped by it, so two agents
/// running this plugin never see each other's.
fn notes_scope() -> &'static str {
    objectiveai_mcp_plugin_framework::identity()
        .agent_instance_hierarchy
        .as_deref()
        .unwrap_or("")
}

/// Whatever your tools need. Every tool receives `&Self`, so put
/// clients, handles and configuration here. It is built once and
/// shared by every call, so anything mutable needs its own interior
/// mutability.
struct Plugin {
    /// The same `Tools` handed to `serve`, so a tool can change the
    /// served set from inside a call. Not a cycle: `Tools` holds route
    /// handlers, never a `Plugin`.
    tools: Arc<Tools<Plugin>>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct GreetArgs {
    /// Who to greet.
    name: String,
}

/// A tool returning `Json<T>` puts `T` in the result's
/// `structured_content` instead of stringifying it, and publishes `T`'s
/// schema alongside the tool — so an agent knows the shape before it
/// calls, and reads fields rather than parsing prose.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct WhoAmI {
    /// The plugin trio the host stamped on this container. `None`
    /// outside a laboratory container, where nothing stamps it.
    plugin_owner: Option<String>,
    plugin_name: Option<String>,
    plugin_version: Option<String>,
    /// The row `agents instances get` returned, verbatim. Embedding the
    /// SDK's own response type rather than copying its fields across
    /// means it cannot drift, and a field added upstream appears here
    /// for free.
    agent: get::ResponseItem,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct NoteWriteArgs {
    /// What to file the note under. Writing the same key twice
    /// replaces it.
    key: String,
    /// The note.
    value: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct NoteReadArgs {
    /// The key given to `scaffold_note_write_deleteme`.
    key: String,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct Note {
    key: String,
    value: String,
    /// RFC3339, and a `String` because it is cast to text in the
    /// query. Decoding a `TIMESTAMPTZ` as a real time type needs
    /// sqlx's `chrono` or `time` feature, which the framework does not
    /// enable — so the cast is what keeps this working with the sqlx
    /// you actually have.
    written_at: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CredentialCallArgs {
    /// The endpoint to call. The credential is sent as its
    /// `Authorization` header.
    url: String,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct CredentialCall {
    /// `"database"` when a stored credential was reused, `"channel"`
    /// when one was asked for. Worth reporting: it is the difference
    /// between a call that bothered a human and one that did not.
    credential_source: String,
    status: u16,
    body: String,
}

/// Returned as a tool result with `is_error`, never as a protocol
/// error, so `step` survives to whoever is reading. A protocol error
/// is for "this call was malformed"; everything below is the call
/// working correctly and the WORLD not cooperating, which an agent can
/// reason about and retry.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct CredentialFailure {
    /// Where it broke: `database`, `publish`, `subscribe`, `open`,
    /// `credential`, `store`, or `request`.
    step: String,
    error: String,
}

fn failed(step: &str, error: impl Into<String>) -> CredentialFailure {
    CredentialFailure {
        step: step.to_string(),
        error: error.into(),
    }
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct Switched {
    /// The tool that just appeared or disappeared.
    tool: String,
    /// Whether it is now being served.
    enabled: bool,
}

/// Both tools are named to be impossible to ship by accident. An agent
/// that can see `..._deleteme` is looking at a plugin whose author
/// never got to the part where they wrote their own tools — which is
/// worth finding out from the tool list rather than from the output.
/// Delete them; they exist to be read once and removed.
#[rmcp::tool_router]
impl Plugin {
    #[rmcp::tool(description = "Scaffold example, delete me. Greets someone by name.")]
    async fn scaffold_greet_deleteme(
        &self,
        Parameters(args): Parameters<GreetArgs>,
    ) -> String {
        format!("Hello, {}!", args.name)
    }

    /// Looks the plugin's OWN agent up, by running `agents instances
    /// get` back through the host.
    ///
    /// Two things worth copying out of this one. The identity the host
    /// stamped on the container is readable before any call arrives,
    /// so a plugin knows which agent it belongs to without being told.
    /// And a plugin can drive the CLI: `command_executor()` sends the
    /// request to the host, which runs it and streams the rows back.
    #[rmcp::tool(
        description = "Scaffold example, delete me. Reports who this plugin is running as."
    )]
    async fn scaffold_whoami_deleteme(&self) -> Result<Json<WhoAmI>, rmcp::ErrorData> {
        let identity = objectiveai_mcp_plugin_framework::identity();

        // Absent outside a laboratory container — `cargo run` on a
        // laptop gets an empty environment, and there is no agent to
        // ask about. An error rather than a half-filled answer: the
        // question has no meaning here, which is different from the
        // agent having nothing to report.
        let Some(hierarchy) = identity.agent_instance_hierarchy.as_deref() else {
            return Err(rmcp::ErrorData::internal_error(
                "no agent instance in the environment — this plugin is not \
                 running inside ObjectiveAI",
                None,
            ));
        };

        // `agents instances get` targets an EXACT agent, addressed as a
        // lineage prefix plus a leaf id. The host stamps the whole
        // hierarchy as one slash-joined string, so split off the last
        // segment; a hierarchy with no slash is a root agent, which has
        // no prefix.
        let (parent, leaf) = match hierarchy.rsplit_once('/') {
            Some((parent, leaf)) => (Some(parent.to_string()), leaf.to_string()),
            None => (None, hierarchy.to_string()),
        };

        let request = get::Request {
            path_type: get::Path::AgentsInstancesGet,
            targets: vec![get::Target::Direct {
                parent_agent_instance_hierarchy: parent,
                agent_instance: leaf,
            }],
            base: Default::default(),
        };

        // The identity argument is `None` on purpose. The HOST decides
        // who a plugin is — it stamps the trio from the image
        // coordinates and refuses any claim off the wire — so a plugin
        // passing its own would be asserting nothing.
        let stream = get::execute(
            &objectiveai_mcp_plugin_framework::command_executor(),
            request,
            None,
        )
        .await
        .map_err(|error| {
            rmcp::ErrorData::internal_error(format!("agents instances get: {error}"), None)
        })?;

        // One target resolves to one row, but the command streams, so
        // take the first and stop rather than assuming a count.
        let agent = match std::pin::pin!(stream).next().await {
            Some(Ok(item)) => item,
            Some(Err(error)) => {
                return Err(rmcp::ErrorData::internal_error(
                    format!("agents instances get: {error}"),
                    None,
                ));
            }
            // An explicitly-named target always yields a row —
            // zero-filled when the agent has no activity — so an empty
            // stream means it does not exist, not that it is idle.
            None => {
                return Err(rmcp::ErrorData::internal_error(
                    format!("no agent instance found for {hierarchy}"),
                    None,
                ));
            }
        };

        Ok(Json(WhoAmI {
            plugin_owner: identity.plugin_owner.clone(),
            plugin_name: identity.plugin_name.clone(),
            plugin_version: identity.plugin_version.clone(),
            agent,
        }))
    }

    /// Calls an endpoint with a credential, asking a person for that
    /// credential exactly once.
    ///
    /// The stored credential is what makes this conditional: the
    /// channel is only published when the database has nothing for
    /// this agent, so the first call may wait on a human and every
    /// call after it will not.
    ///
    /// Returns failures as a tool RESULT rather than a protocol error
    /// — see [`CredentialFailure`].
    #[rmcp::tool(
        description = "Scaffold example, delete me. Calls a URL with a credential, \
                       asking for one through a viewer channel if none is stored."
    )]
    async fn scaffold_credential_call_deleteme(
        &self,
        Parameters(args): Parameters<CredentialCallArgs>,
    ) -> rmcp::model::CallToolResult {
        match credential_call(&args.url).await {
            Ok(call) => match serde_json::to_value(&call) {
                Ok(value) => rmcp::model::CallToolResult::structured(value),
                Err(error) => rmcp::model::CallToolResult::structured_error(
                    serde_json::json!({ "step": "encode", "error": error.to_string() }),
                ),
            },
            Err(failure) => rmcp::model::CallToolResult::structured_error(
                serde_json::json!({ "step": failure.step, "error": failure.error }),
            ),
        }
    }

    /// Writes a note to the plugin's database, replacing any note
    /// already under that key.
    #[rmcp::tool(
        description = "Scaffold example, delete me. Stores a note under a key."
    )]
    async fn scaffold_note_write_deleteme(
        &self,
        Parameters(args): Parameters<NoteWriteArgs>,
    ) -> Result<Json<Note>, rmcp::ErrorData> {
        let pool = notes_pool().await?;

        // Bound parameters, never formatted into the string. `$1` is
        // sent to Postgres as DATA, so a note whose value is
        // `'; DROP TABLE ...` is just an odd note.
        let row = sqlx::query(
            "
            INSERT INTO scaffold_notes_deleteme
                (agent_instance_hierarchy, note_key, note_value)
            VALUES ($1, $2, $3)
            ON CONFLICT (agent_instance_hierarchy, note_key) DO UPDATE
                SET note_value = EXCLUDED.note_value, written_at = now()
            RETURNING note_value, written_at::text AS written_at
            ",
        )
        .bind(notes_scope())
        .bind(&args.key)
        .bind(&args.value)
        .fetch_one(&pool)
        .await
        .map_err(|error| {
            rmcp::ErrorData::internal_error(error_chain("write the note", &error), None)
        })?;

        Ok(Json(Note {
            key: args.key,
            value: row.get("note_value"),
            written_at: row.get("written_at"),
        }))
    }

    /// Reads back what `scaffold_note_write_deleteme` stored.
    #[rmcp::tool(
        description = "Scaffold example, delete me. Reads the note stored under a key."
    )]
    async fn scaffold_note_read_deleteme(
        &self,
        Parameters(args): Parameters<NoteReadArgs>,
    ) -> Result<Json<Note>, rmcp::ErrorData> {
        let pool = notes_pool().await?;

        let row = sqlx::query(
            "
            SELECT note_value, written_at::text AS written_at
            FROM scaffold_notes_deleteme
            WHERE agent_instance_hierarchy = $1 AND note_key = $2
            ",
        )
        .bind(notes_scope())
        .bind(&args.key)
        // `fetch_optional`, not `fetch_one`: no note under that key is
        // an ordinary answer, and would otherwise arrive as
        // `RowNotFound` dressed up as a database failure.
        .fetch_optional(&pool)
        .await
        .map_err(|error| {
            rmcp::ErrorData::internal_error(error_chain("read the note", &error), None)
        })?;

        let Some(row) = row else {
            return Err(rmcp::ErrorData::invalid_params(
                format!("no note stored under {:?}", args.key),
                None,
            ));
        };

        Ok(Json(Note {
            key: args.key,
            value: row.get("note_value"),
            written_at: row.get("written_at"),
        }))
    }

    /// Flips the second tool on or off, and the agent's tool list
    /// changes underneath it.
    ///
    /// This tool ITSELF only exists when the agent declared the
    /// `switch` argument — a plugin whose arguments did not ask for
    /// the feature serves neither of the pair, and nothing here can
    /// talk it into existing.
    #[rmcp::tool(
        description = "Scaffold example, delete me. Toggles whether a second tool is served."
    )]
    async fn scaffold_switch_deleteme(&self) -> Json<Switched> {
        // The served list IS the state. Keeping a separate flag would
        // create a second source of truth that could disagree with what
        // is actually routed.
        let currently_on = self
            .tools
            .routes()
            .iter()
            .any(|route| route.attr.name.as_ref() == SWITCHED_TOOL);
        let enabled = !currently_on;

        // Swaps the served set AND sends
        // `notifications/tools/list_changed`, so a client that re-lists
        // on the notification sees the new set — the store lands before
        // the notification goes out.
        self.tools.replace(served_routes(enabled));

        Json(Switched {
            tool: SWITCHED_TOOL.to_string(),
            enabled,
        })
    }

    /// Not served until `scaffold_switch_deleteme` switches it on, so
    /// an agent that lists tools at startup will not see it at all.
    ///
    /// Note it is declared exactly like any other tool. "Conditional"
    /// is not a property of the tool — it is a property of the route
    /// list `served_routes` builds.
    #[rmcp::tool(
        description = "Scaffold example, delete me. Exists only while switched on."
    )]
    async fn scaffold_switched_deleteme(&self) -> String {
        "I did not exist when you listed tools.".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<Infallible, std::io::Error> {
    // The starting set: the argument gate has already been applied, and
    // the switched tool starts off. A plugin whose tools never change
    // can pass `Plugin::tool_router()` straight in and never think
    // about this again.
    let tools = Tools::new(served_routes(false));

    // The plugin holds the same handle, so a tool can swap the set from
    // inside a call. `serve` takes ownership of the state and the
    // `Arc`, hence the clone.
    let plugin = Plugin {
        tools: Arc::clone(&tools),
    };

    objectiveai_mcp_plugin_framework::serve::serve(
        objectiveai_mcp_plugin_framework::config::Config::new(PORT, NAME, VERSION)
            .with_description("Starting point for an ObjectiveAI MCP plugin.")
            .with_instructions("Replace this with what an agent should know."),
        plugin,
        tools,
    )
    .await
}
