//! `viewer send` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub path: String,
    pub body: serde_json::Value,
    pub jq: Option<String>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "viewer".to_string(),
            "send".to_string(),
            self.path.clone(),
            serde_json::to_string(&self.body).expect("body serializes"),
        ];
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub status: u16,
    pub body: serde_json::Value,
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["viewer", "send", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["viewer", "send", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
