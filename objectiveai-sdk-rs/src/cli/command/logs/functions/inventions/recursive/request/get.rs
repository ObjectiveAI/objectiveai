//! `logs functions inventions recursive request get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
    pub jq: Option<String>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "functions", "inventions", "recursive", "request", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

pub type Response = crate::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParamsLog;

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["logs", "functions", "inventions", "recursive", "request", "get", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["logs", "functions", "inventions", "recursive", "request", "get", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
