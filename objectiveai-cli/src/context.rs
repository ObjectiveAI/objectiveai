//! Process-wide state threaded into every bare-naked `command::*::execute`.
//!
//! `Context` is constructed once in `main.rs` (or by a programmatic
//! embedder), then borrowed into the command tree. Bodies that need IO
//! grab a `filesystem::Client` or an HTTP client off it rather than
//! building one per call.

use crate::filesystem;
use crate::run::Config;

pub struct Context {
    pub config: Config,
    pub filesystem: filesystem::Client,
}

impl Context {
    pub fn new(config: Config) -> Self {
        let base_dir = config.config_base_dir.clone();
        let filesystem = filesystem::Client::new(
            base_dir,
            config.commit_author_name.clone(),
            config.commit_author_email.clone(),
        );
        Self { config, filesystem }
    }
}
