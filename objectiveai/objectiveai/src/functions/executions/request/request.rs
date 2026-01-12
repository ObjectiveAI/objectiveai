use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Request {
    FunctionInlineProfileInline {
        body: super::FunctionInlineProfileInlineRequestBody,
    },
    FunctionInlineProfileRemote {
        path: super::FunctionInlineProfileRemoteRequestPath,
        body: super::FunctionInlineProfileRemoteRequestBody,
    },
    FunctionRemoteProfileInline {
        path: super::FunctionRemoteProfileInlineRequestPath,
        body: super::FunctionRemoteProfileInlineRequestBody,
    },
    FunctionRemoteProfileRemote {
        path: super::FunctionRemoteProfileRemoteRequestPath,
        body: super::FunctionRemoteProfileRemoteRequestBody,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionExecutionCreateParams {
    FunctionInlineProfileInline(super::FunctionInlineProfileInlineRequestBody),
    FunctionInlineProfileRemote(super::FunctionInlineProfileRemoteRequestBody),
    FunctionRemoteProfileInline(super::FunctionRemoteProfileInlineRequestBody),
    FunctionRemoteProfileRemote(super::FunctionRemoteProfileRemoteRequestBody),
}
