//! Function execution request types.

use crate::functions;
use serde::{Deserialize, Serialize};

/// Internal request representation with path and body separated.
///
/// Used internally to route requests to the appropriate API endpoint.
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

impl Request {
    pub fn remote_function(&self) -> Option<(&str, &str, Option<&str>)> {
        match self {
            Request::FunctionRemoteProfileInline { path, .. } => Some((
                path.fowner.as_str(),
                path.frepository.as_str(),
                path.fcommit.as_deref(),
            )),
            Request::FunctionRemoteProfileRemote { path, .. } => Some((
                path.fowner.as_str(),
                path.frepository.as_str(),
                path.fcommit.as_deref(),
            )),
            _ => None,
        }
    }

    pub fn inline_function(&self) -> Option<&functions::InlineFunction> {
        match self {
            Request::FunctionInlineProfileInline { body } => {
                Some(&body.function)
            }
            Request::FunctionInlineProfileRemote { body, .. } => {
                Some(&body.function)
            }
            _ => None,
        }
    }

    pub fn remote_profile(&self) -> Option<(&str, &str, Option<&str>)> {
        match self {
            Request::FunctionInlineProfileRemote { path, .. } => Some((
                path.powner.as_str(),
                path.prepository.as_str(),
                path.pcommit.as_deref(),
            )),
            Request::FunctionRemoteProfileRemote { path, .. } => Some((
                path.powner.as_str(),
                path.prepository.as_str(),
                path.pcommit.as_deref(),
            )),
            _ => None,
        }
    }

    pub fn inline_profile(&self) -> Option<&functions::InlineProfile> {
        match self {
            Request::FunctionInlineProfileInline { body } => {
                Some(&body.profile)
            }
            Request::FunctionRemoteProfileInline { body, .. } => {
                Some(&body.profile)
            }
            _ => None,
        }
    }

    pub fn base(&self) -> &super::FunctionRemoteProfileRemoteRequestBody {
        match self {
            Request::FunctionInlineProfileInline { body } => &body.base,
            Request::FunctionInlineProfileRemote { body, .. } => &body.base,
            Request::FunctionRemoteProfileInline { body, .. } => &body.base,
            Request::FunctionRemoteProfileRemote { body, .. } => body,
        }
    }

    pub fn base_mut(
        &mut self,
    ) -> &mut super::FunctionRemoteProfileRemoteRequestBody {
        match self {
            Request::FunctionInlineProfileInline { body } => &mut body.base,
            Request::FunctionInlineProfileRemote { body, .. } => &mut body.base,
            Request::FunctionRemoteProfileInline { body, .. } => &mut body.base,
            Request::FunctionRemoteProfileRemote { body, .. } => body,
        }
    }
}

/// Parameters for creating a function execution.
///
/// Supports four combinations based on whether the Function and Profile
/// are provided inline or referenced from GitHub repositories.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionExecutionCreateParams {
    /// Inline Function with inline Profile.
    FunctionInlineProfileInline(super::FunctionInlineProfileInlineRequestBody),
    /// Inline Function with remote Profile.
    FunctionInlineProfileRemote(super::FunctionInlineProfileRemoteRequestBody),
    /// Remote Function with inline Profile.
    FunctionRemoteProfileInline(super::FunctionRemoteProfileInlineRequestBody),
    /// Remote Function with remote Profile.
    FunctionRemoteProfileRemote(super::FunctionRemoteProfileRemoteRequestBody),
}

impl FunctionExecutionCreateParams {
    pub fn inline_function(&self) -> Option<&functions::InlineFunction> {
        match self {
            FunctionExecutionCreateParams::FunctionInlineProfileInline(
                body,
            ) => Some(&body.function),
            FunctionExecutionCreateParams::FunctionInlineProfileRemote(
                body,
            ) => Some(&body.function),
            _ => None,
        }
    }

    pub fn inline_profile(&self) -> Option<&functions::InlineProfile> {
        match self {
            FunctionExecutionCreateParams::FunctionInlineProfileInline(
                body,
            ) => Some(&body.profile),
            FunctionExecutionCreateParams::FunctionRemoteProfileInline(
                body,
            ) => Some(&body.profile),
            _ => None,
        }
    }

    pub fn base(&self) -> &super::FunctionRemoteProfileRemoteRequestBody {
        match self {
            FunctionExecutionCreateParams::FunctionInlineProfileInline(
                body,
            ) => &body.base,
            FunctionExecutionCreateParams::FunctionInlineProfileRemote(
                body,
            ) => &body.base,
            FunctionExecutionCreateParams::FunctionRemoteProfileInline(
                body,
            ) => &body.base,
            FunctionExecutionCreateParams::FunctionRemoteProfileRemote(
                body,
            ) => body,
        }
    }

    pub fn base_mut(
        &mut self,
    ) -> &mut super::FunctionRemoteProfileRemoteRequestBody {
        match self {
            FunctionExecutionCreateParams::FunctionInlineProfileInline(
                body,
            ) => &mut body.base,
            FunctionExecutionCreateParams::FunctionInlineProfileRemote(
                body,
            ) => &mut body.base,
            FunctionExecutionCreateParams::FunctionRemoteProfileInline(
                body,
            ) => &mut body.base,
            FunctionExecutionCreateParams::FunctionRemoteProfileRemote(
                body,
            ) => body,
        }
    }
}
