use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use indexmap::IndexMap;

pub type CallInventionTool = Arc<
    dyn Fn(
            serde_json::Value,
        )
            -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct InventionTool {
    /// Tool name with `.` normalised to `_` (done once in the
    /// constructor, so every downstream consumer — the rmcp `Tool`
    /// shown to the agent, the orchestrator's expected-tool-name list,
    /// the `tool_key` hash — sees a single canonical form).
    pub name: String,
    pub description: &'static str,
    pub parameters: IndexMap<String, serde_json::Value>,
    pub call: CallInventionTool,
}

impl InventionTool {
    pub fn tool_key(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name.hash(&mut h);
        self.description.hash(&mut h);
        for (k, v) in &self.parameters {
            k.hash(&mut h);
            v.to_string().hash(&mut h);
        }
        format!("{:x}", h.finish())
    }

    pub fn new_sync<T: super::JsonSchema>(
        name: impl Into<String>,
        description: &'static str,
        f: impl Fn(serde_json::Value) -> Result<String, String>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            name: name.into().replace('.', "_"),
            description,
            parameters: T::indexmap_json_schema(),
            call: Arc::new(move |args| {
                let result = f(args);
                Box::pin(async move { result })
            }),
        }
    }

    pub fn new_async<T: super::JsonSchema>(
        name: impl Into<String>,
        description: &'static str,
        f: impl Fn(
            serde_json::Value,
        )
            -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            name: name.into().replace('.', "_"),
            description,
            parameters: T::indexmap_json_schema(),
            call: Arc::new(move |args| f(args)),
        }
    }
}
