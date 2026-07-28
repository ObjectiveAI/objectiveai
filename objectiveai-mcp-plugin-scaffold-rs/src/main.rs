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
use objectiveai_sdk::cli::command::agents::instances::get;
use rmcp::handler::server::tool::ToolRoute;
use rmcp::handler::server::wrapper::{Json, Parameters};
use sqlx::Row as _;

/// Must match `mcp.port` in `objectiveai.json`.
const PORT: u16 = 8080;
/// The routing prefix ObjectiveAI derives — see the module docs.
const NAME: &str = "objectiveai-mcp-plugin-scaffold";
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

/// The pool, with the table guaranteed to exist.
///
/// Note the queries are `sqlx::query`, not the `sqlx::query!` MACRO.
/// The macro checks SQL against a live database AT COMPILE TIME, which
/// would make this plugin unbuildable without one — and the database a
/// plugin talks to does not exist until a host creates its container.
async fn notes_pool() -> Result<db::Pool, rmcp::ErrorData> {
    let pool = db::connect(Default::default())
        .await
        .map_err(|error| database_error("connect", &error))?;

    NOTES_TABLE
        .get_or_try_init(|| async {
            sqlx::query(CREATE_NOTES).execute(&pool).await.map(|_| ())
        })
        .await
        .map_err(|error| database_error("create the notes table", &error))?;

    Ok(pool)
}

/// `source` chains are where the actual cause lives — sqlx's top-level
/// `Display` is often just "error returned from database server". An
/// agent gets one string, so it has to be the whole story.
fn database_error(doing: &str, error: &dyn std::error::Error) -> rmcp::ErrorData {
    let mut message = format!("{doing}: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    rmcp::ErrorData::internal_error(message, None)
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
        .map_err(|error| database_error("write the note", &error))?;

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
        .map_err(|error| database_error("read the note", &error))?;

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
