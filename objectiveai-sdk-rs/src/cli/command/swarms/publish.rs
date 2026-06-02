//! `swarms publish` — async handler stub.

use crate::swarm::RemoteSwarmBase;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub repository: String,
    pub body: RequestBody,
    pub message: RequestPublishMessage,
    pub overwrite: bool,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestBody {
    Inline(RemoteSwarmBase),
    File(std::path::PathBuf),
    PythonInline(String),
    PythonFile(std::path::PathBuf),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestPublishMessage {
    Inline(String),
    File(std::path::PathBuf),
}

impl RequestBody {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestBody::Inline(v) => {
                out.push("--body-inline".to_string());
                out.push(serde_json::to_string(v).expect("body serializes"));
            }
            RequestBody::File(p) => {
                out.push("--body-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            RequestBody::PythonInline(code) => {
                out.push("--body-python-inline".to_string());
                out.push(code.clone());
            }
            RequestBody::PythonFile(p) => {
                out.push("--body-python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl RequestPublishMessage {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestPublishMessage::Inline(s) => {
                out.push("--message-inline".to_string());
                out.push(s.clone());
            }
            RequestPublishMessage::File(p) => {
                out.push("--message-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "swarms".to_string(),
            "publish".to_string(),
            "--repository".to_string(),
            self.repository.clone(),
        ];
        self.body.push_flags(&mut argv);
        self.message.push_flags(&mut argv);
        if self.overwrite {
            argv.push("--overwrite".to_string());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub sha: String,
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["swarms", "publish", "--request-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["swarms", "publish", "--response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;
}
