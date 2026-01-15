use crate::functions;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Request {
    FunctionInline {
        body: super::FunctionInlineRequestBody,
    },
    FunctionRemote {
        path: super::FunctionRemoteRequestPath,
        body: super::FunctionRemoteRequestBody,
    },
}

impl Request {
    pub fn remote_function(&self) -> Option<(&str, &str, Option<&str>)> {
        match self {
            Request::FunctionRemote { path, .. } => {
                Some((&path.fowner, &path.frepository, path.fcommit.as_deref()))
            }
            _ => None,
        }
    }

    pub fn inline_function(&self) -> Option<&functions::InlineFunction> {
        match self {
            Request::FunctionInline { body } => Some(&body.function),
            _ => None,
        }
    }

    pub fn base(&self) -> &super::FunctionRemoteRequestBody {
        match self {
            Request::FunctionInline { body } => &body.base,
            Request::FunctionRemote { body, .. } => &body,
        }
    }

    pub fn base_mut(&mut self) -> &mut super::FunctionRemoteRequestBody {
        match self {
            Request::FunctionInline { body } => &mut body.base,
            Request::FunctionRemote { body, .. } => body,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionProfileComputationCreateParams {
    FunctionInline(super::FunctionInlineRequestBody),
    FunctionRemote(super::FunctionRemoteRequestBody),
}

impl FunctionProfileComputationCreateParams {
    pub fn inline_function(&self) -> Option<&functions::InlineFunction> {
        match self {
            FunctionProfileComputationCreateParams::FunctionInline(body) => {
                Some(&body.function)
            }
            _ => None,
        }
    }

    pub fn base(&self) -> &super::FunctionRemoteRequestBody {
        match self {
            FunctionProfileComputationCreateParams::FunctionInline(body) => {
                &body.base
            }
            FunctionProfileComputationCreateParams::FunctionRemote(body) => {
                body
            }
        }
    }

    pub fn base_mut(&mut self) -> &mut super::FunctionRemoteRequestBody {
        match self {
            FunctionProfileComputationCreateParams::FunctionInline(body) => {
                &mut body.base
            }
            FunctionProfileComputationCreateParams::FunctionRemote(body) => {
                body
            }
        }
    }
}
