//! An ObjectiveAI MCP plugin, in as few moving parts as one can have.
//!
//! Everything specific to running inside ObjectiveAI — the transport,
//! the port binding, the `initialize` reply, the command extension —
//! is [`objectiveai_mcp_plugin_framework`]'s job. What is left here is
//! the part that is actually yours: the tools, and what they do.
//!
//! Four things to change when you copy this:
//!
//! 1. `NAME` and `VERSION` below, and the `[package]` name in
//!    `Cargo.toml`. The NAME is not cosmetic — ObjectiveAI derives the
//!    routing prefix from it, so it becomes part of every tool name
//!    the agent sees.
//! 2. `PORT`, here and in `objectiveai.json`. They must agree; the
//!    host publishes the port the manifest names.
//! 3. The binary name in `Cargo.toml` and `Containerfile`.
//! 4. The tools.
//!
//! And drop the `path = ` from the framework dependency — it exists
//! only so this compiles inside the monorepo.

use std::convert::Infallible;

use objectiveai_mcp_plugin_framework as oai;
// Brings `rmcp` into scope under the name the `#[tool_router]` and
// `#[tool]` macros expand to. Depending on `rmcp` separately would
// risk two versions in one binary, where a `ToolRouter` built by the
// macros would not fit `serve`.
use oai::rmcp;
use rmcp::handler::server::wrapper::Parameters;

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

#[rmcp::tool_router]
impl Plugin {
    #[rmcp::tool(description = "Greet someone by name.")]
    async fn greet(&self, Parameters(args): Parameters<GreetArgs>) -> String {
        format!("Hello, {}!", args.name)
    }

    /// Proves the ambient context is readable — the identity the host
    /// stamped on this container, and whatever the agent configured
    /// for this plugin.
    #[rmcp::tool(description = "Report who this plugin is running as.")]
    async fn whoami(&self) -> String {
        let identity = oai::identity();
        let plugin = identity.plugin_name.as_deref().unwrap_or("(not a plugin)");
        let arguments: Vec<&str> =
            oai::arguments().keys().map(String::as_str).collect();
        format!("plugin={plugin} arguments={arguments:?}")
    }
}

#[tokio::main]
async fn main() -> Result<Infallible, std::io::Error> {
    // The tool set is swappable while serving: keep a clone of this
    // and call `replace` to change what the agent can see, and the
    // framework re-lists it for you. A plugin whose tools never change
    // simply never calls it.
    let tools = oai::tools::Tools::new(Plugin::tool_router());

    oai::serve::serve(
        oai::config::Config::new(PORT, NAME, VERSION)
            .with_description("Starting point for an ObjectiveAI MCP plugin.")
            .with_instructions("Call greet to greet someone by name."),
        Plugin,
        tools,
    )
    .await
}
