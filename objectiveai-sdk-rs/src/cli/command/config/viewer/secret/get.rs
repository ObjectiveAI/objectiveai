//! `config viewer secret get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub filter: Option<String>,
    pub jq: Option<String>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["config".to_string(), "viewer".to_string(), "secret".to_string(), "get".to_string()];
        if let Some(filter) = &self.filter {
            argv.push(filter.clone());
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["config", "viewer", "secret", "get", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["config", "viewer", "secret", "get", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
