//! Process-wide state threaded into every bare-naked `command::*::execute`.
//!
//! `Context` is constructed once in `main.rs` (or by a programmatic
//! embedder), then borrowed into the command tree. Bodies that need IO
//! grab a `filesystem::Client` or the prebuilt `HttpClient` off it
//! rather than building one per call.

use objectiveai_sdk::HttpClient;

use crate::filesystem;
use crate::run::Config;

pub struct Context {
    pub config: Config,
    pub filesystem: filesystem::Client,
    pub http: HttpClient,
}

impl Context {
    pub fn new(config: Config) -> Self {
        let base_dir = config.config_base_dir.clone();
        let filesystem = filesystem::Client::new(
            base_dir,
            config.commit_author_name.clone(),
            config.commit_author_email.clone(),
        );
        // The SDK's `env` feature lets `HttpClient::new` fall back to
        // env vars for any `None` argument (OBJECTIVEAI_ADDRESS,
        // OBJECTIVEAI_AUTHORIZATION, GITHUB_AUTHORIZATION, ...). We
        // explicitly thread the agent_instance_hierarchy from cli
        // config because it's a config field, not a raw env value.
        let http = HttpClient::new(
            reqwest::Client::new(),
            None::<String>,
            None::<String>,
            None::<String>,
            None::<String>,
            None::<String>,
            None::<String>,
            None::<String>,
            None,
            None::<String>,
            None::<String>,
            config.commit_author_name.clone(),
            config.commit_author_email.clone(),
            Some(config.agent_instance_hierarchy.clone()),
            config.mcp_session_id.clone(),
        );
        Self {
            config,
            filesystem,
            http,
        }
    }
}
