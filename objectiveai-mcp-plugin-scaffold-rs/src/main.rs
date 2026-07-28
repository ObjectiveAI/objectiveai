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

// Brings `rmcp` into scope under the name the `#[tool_router]` and
// `#[tool]` macros expand to. Depending on `rmcp` separately would
// risk two versions in one binary, where a `ToolRouter` built by the
// macros would not fit `serve`.
use objectiveai_mcp_plugin_framework::rmcp;
// Likewise the SDK, whose `CommandExecutor` trait every `execute` is
// generic over: a separately-resolved copy would be a different trait.
use objectiveai_mcp_plugin_framework::objectiveai_sdk;
use futures::StreamExt;
use objectiveai_sdk::cli::command::agents::instances::get;
use rmcp::handler::server::wrapper::{Json, Parameters};

/// Must match `mcp.port` in `objectiveai.json`.
const PORT: u16 = 8080;
/// The routing prefix ObjectiveAI derives — see the module docs.
const NAME: &str = "objectiveai-mcp-plugin-scaffold";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whatever your tools need. Every tool receives `&Self`, so put
/// clients, handles and configuration here. It is built once and
/// shared by every call, so anything mutable needs its own interior
/// mutability.
struct Plugin;

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
}

#[tokio::main]
async fn main() -> Result<Infallible, std::io::Error> {
    // The tool set is swappable while serving: keep a clone of this
    // and call `replace` to change what the agent can see, and the
    // framework re-lists it for you. A plugin whose tools never change
    // simply never calls it.
    let tools =
        objectiveai_mcp_plugin_framework::tools::Tools::new(Plugin::tool_router());

    objectiveai_mcp_plugin_framework::serve::serve(
        objectiveai_mcp_plugin_framework::config::Config::new(PORT, NAME, VERSION)
            .with_description("Starting point for an ObjectiveAI MCP plugin.")
            .with_instructions("Replace this with what an agent should know."),
        Plugin,
        tools,
    )
    .await
}
