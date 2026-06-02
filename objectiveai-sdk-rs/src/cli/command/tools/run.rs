//! `tools run` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub name: String,
    pub args: Vec<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "tools".to_string(),
            "run".to_string(),
            self.name.clone(),
        ];
        argv.extend(self.args.iter().cloned());
        argv
    }
}
