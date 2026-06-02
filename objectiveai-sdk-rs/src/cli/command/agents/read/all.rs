//! `agents read all` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub agent_instance_hierarchies: Vec<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "read".to_string(),
            "all".to_string(),
        ];
        argv.extend(self.agent_instance_hierarchies.iter().cloned());
        argv
    }
}

pub struct ResponseItem {
    pub agent_id: String,
    pub items: Vec<crate::filesystem::logs::queue::QueueItem>,
}
