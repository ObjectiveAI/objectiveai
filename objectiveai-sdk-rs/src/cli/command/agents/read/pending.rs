//! `agents read pending` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub agent_instance_hierarchies: Vec<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "read".to_string(),
            "pending".to_string(),
        ];
        argv.extend(self.agent_instance_hierarchies.iter().cloned());
        argv
    }
}
