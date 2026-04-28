//! Binary entry point for the test upstream. Tests typically use the lib
//! directly via `spawn_test_upstream`, but the binary also exists so the
//! TypeScript test runner can spawn it as a subprocess.

use std::net::SocketAddr;

use envconfig::Envconfig;
use test_upstream::{Config, TestResource, TestTool, spawn_test_upstream};

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "ADDRESS")]
    address: Option<String>,
    #[envconfig(from = "PORT")]
    port: Option<u16>,
    #[envconfig(from = "SERVER_NAME")]
    server_name: Option<String>,
    #[envconfig(from = "INITIAL_TOOLS_JSON")]
    initial_tools_json: Option<String>,
    #[envconfig(from = "INITIAL_RESOURCES_JSON")]
    initial_resources_json: Option<String>,
    #[envconfig(from = "REQUIRE_AUTH")]
    require_auth: Option<String>,
    #[envconfig(from = "HEADER_GATE_NAME")]
    header_gate_name: Option<String>,
    #[envconfig(from = "HEADER_GATE_VALUE")]
    header_gate_value: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let _ = dotenv::dotenv();
    let env = EnvConfigBuilder::init_from_env().unwrap_or(EnvConfigBuilder {
        address: None,
        port: None,
        server_name: None,
        initial_tools_json: None,
        initial_resources_json: None,
        require_auth: None,
        header_gate_name: None,
        header_gate_value: None,
    });

    let address: SocketAddr = format!(
        "{}:{}",
        env.address.unwrap_or_else(|| "127.0.0.1".into()),
        env.port.unwrap_or(0),
    )
    .parse()?;

    let initial_tools: Vec<TestTool> = match env.initial_tools_json {
        Some(json) => serde_json::from_str(&json)?,
        None => Vec::new(),
    };
    let initial_resources: Vec<TestResource> = match env.initial_resources_json {
        Some(json) => serde_json::from_str(&json)?,
        None => Vec::new(),
    };

    let header_gate = match (env.header_gate_name, env.header_gate_value) {
        (Some(n), Some(v)) => Some((n, v)),
        _ => None,
    };

    let config = Config {
        address,
        server_name: env.server_name.unwrap_or_else(|| "test-upstream".into()),
        initial_tools,
        initial_resources,
        require_auth: env.require_auth,
        header_gate,
    };

    let handle = spawn_test_upstream(config).await?;
    tracing::info!("test upstream listening on {}", handle.address);
    handle.serve_task.await??;
    Ok(())
}
