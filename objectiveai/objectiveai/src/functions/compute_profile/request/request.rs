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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionComputeProfileCreateParams {
    FunctionInline(super::FunctionInlineRequestBody),
    FunctionRemote(super::FunctionRemoteRequestBody),
}
