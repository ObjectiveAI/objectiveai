//! JSON body input — inline only. cli-stream is a per-stream
//! subprocess; the spawning embedder always has the body in memory,
//! so a file path indirection would be a useless extra step.

use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct BodySource {
    /// Inline JSON body.
    #[arg(long)]
    pub body: String,
}

impl BodySource {
    /// Parse the inline body into `T`.
    pub fn resolve<T: serde::de::DeserializeOwned>(self) -> Result<T, String> {
        let mut de = serde_json::Deserializer::from_str(&self.body);
        serde_path_to_error::deserialize(&mut de)
            .map_err(|e| format!("--body parse error at `{}`: {}", e.path(), e.inner()))
    }
}
