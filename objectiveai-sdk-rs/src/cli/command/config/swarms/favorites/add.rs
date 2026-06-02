//! `config swarms favorites add` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub name: String,
    pub path: String,
    pub note: String,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "config".to_string(),
            "swarms".to_string(),
            "favorites".to_string(),
            "add".to_string(),
            "--name".to_string(),
            self.name.clone(),
            "--path".to_string(),
            self.path.clone(),
            "--note".to_string(),
            self.note.clone(),
        ]
    }
}

pub type Response = crate::cli::command::Ok;

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["config", "swarms", "favorites", "add", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["config", "swarms", "favorites", "add", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
