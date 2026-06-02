//! `config swarms favorites del` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub name: String,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["config".to_string(), "swarms".to_string(), "favorites".to_string(), "del".to_string(), self.name.clone()]
    }
}

pub type Response = crate::cli::command::Ok;

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["config", "swarms", "favorites", "del", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["config", "swarms", "favorites", "del", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
