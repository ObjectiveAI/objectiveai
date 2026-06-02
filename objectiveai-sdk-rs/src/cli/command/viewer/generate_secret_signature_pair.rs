//! `viewer generate-secret-signature-pair` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request;

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["viewer".to_string(), "generate-secret-signature-pair".to_string()]
    }
}
