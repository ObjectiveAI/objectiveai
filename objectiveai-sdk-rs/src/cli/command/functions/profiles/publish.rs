//! `functions profiles publish` — async handler stub.

use crate::functions::RemoteProfile;
use crate::cli::command::IntoCommand;

pub struct Request {
    pub repository: String,
    pub body: RequestBody,
    pub message: RequestPublishMessage,
    pub overwrite: bool,
}

pub enum RequestBody {
    Inline(RemoteProfile),
    File(std::path::PathBuf),
    PythonInline(String),
    PythonFile(std::path::PathBuf),
}

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

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "functions".to_string(), "profiles".to_string(),
            "publish".to_string(),
            "--repository".to_string(),
            self.repository.clone(),
        ];
        self.body.push_flags(&mut argv);
        self.message.push_flags(&mut argv);
        if self.overwrite {
            argv.push("--overwrite".to_string());
        }
        argv
    }
}
