//! The agent's schema: what this image's agent value may be, told to
//! the proxy so a server can ask.

use serde_json::Value;

use crate::{Client, Error};

impl Client {
    /// Post the JSON Schema of the agent value this image accepts.
    ///
    /// One `POST` to `/agent-schema/agent`; the proxy keeps the latest
    /// and answers every server that asks with it. An image that never
    /// posts one is answered "no schema", which is allowed — a schema
    /// is a courtesy, not an obligation.
    pub async fn agent_schema_set(&self, schema: &Value) -> Result<(), Error> {
        let response = self
            .http()
            .post(crate::url("/agent-schema/agent"))
            .json(schema)
            .send()
            .await
            .map_err(Error::AgentSchemaRequest)?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::AgentSchemaStatus(status.as_u16()));
        }
        Ok(())
    }
}
