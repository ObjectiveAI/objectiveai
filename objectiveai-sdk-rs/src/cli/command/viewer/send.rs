//! `viewer send` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub path: String,
    pub body: serde_json::Value,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "viewer".to_string(),
            "send".to_string(),
            self.path.clone(),
            serde_json::to_string(&self.body).expect("body serializes"),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    pub status: u16,
    pub body: serde_json::Value,
}
