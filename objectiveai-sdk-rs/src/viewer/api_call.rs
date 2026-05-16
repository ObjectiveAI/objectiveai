//! Types for the viewer's `Event::ApiCall` bridge.
//!
//! In viewer mode, the JS SDK's `ObjectiveAI` client diverts every HTTP
//! method through a Tauri postMessage channel instead of `fetch()`. The
//! iframe posts `{kind: "api-call-invoke", sub_type, body}` to the host,
//! which dispatches `sub_type` to the matching upstream call and emits
//! [`Event::ApiCall`](super::Event::ApiCall) chunks back to the iframe.
//!
//! [`ApiCallSubType`] enumerates every (HTTP method, path) tuple the
//! `objectiveai-api` crate exposes. The serde rename of each variant is
//! `"<METHOD>_<PATH>"` (e.g. `"POST_/agent/completions"`,
//! `"GET_/auth/keys"`) so the wire format is unambiguous when the same
//! path serves multiple methods (notably `/auth/keys`, which has POST,
//! DELETE, and GET).
//!
//! An AST coverage test (`tests/api_routes_coverage.rs`) diffs this
//! enum against `objectiveai-api/src/run.rs`'s `Router::new()` chain;
//! adding a new route to the api crate without a matching variant
//! fails the test.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One `(method, path)` tuple — one variant per route the
/// `objectiveai-api` crate's `Router::new()` chain declares.
///
/// Serializes as `"<METHOD>_<PATH>"`. The underscore separator (rather
/// than space) keeps the value safe to use as a Tauri event channel
/// suffix and as a JS object key.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[schemars(rename = "viewer.ApiCallSubType")]
pub enum ApiCallSubType {
    #[serde(rename = "POST_/agent/completions")]
    PostAgentCompletions,
    #[serde(rename = "POST_/vector/completions")]
    PostVectorCompletions,
    #[serde(rename = "POST_/vector/completions/votes")]
    PostVectorCompletionsVotes,
    #[serde(rename = "POST_/vector/completions/cache")]
    PostVectorCompletionsCache,
    #[serde(rename = "POST_/functions/list")]
    PostFunctionsList,
    #[serde(rename = "POST_/functions")]
    PostFunctions,
    #[serde(rename = "POST_/functions/usage")]
    PostFunctionsUsage,
    #[serde(rename = "POST_/functions/executions")]
    PostFunctionsExecutions,
    #[serde(rename = "POST_/functions/profiles/list")]
    PostFunctionsProfilesList,
    #[serde(rename = "POST_/functions/profiles")]
    PostFunctionsProfiles,
    #[serde(rename = "POST_/functions/profiles/usage")]
    PostFunctionsProfilesUsage,
    #[serde(rename = "POST_/functions/profiles/pairs/list")]
    PostFunctionsProfilesPairsList,
    #[serde(rename = "POST_/functions/profiles/pairs/usage")]
    PostFunctionsProfilesPairsUsage,
    #[serde(rename = "POST_/functions/inventions")]
    PostFunctionsInventions,
    #[serde(rename = "POST_/functions/inventions/recursive")]
    PostFunctionsInventionsRecursive,
    #[serde(rename = "POST_/functions/inventions/prompts/list")]
    PostFunctionsInventionsPromptsList,
    #[serde(rename = "POST_/functions/inventions/prompts")]
    PostFunctionsInventionsPrompts,
    #[serde(rename = "POST_/functions/inventions/prompts/usage")]
    PostFunctionsInventionsPromptsUsage,
    #[serde(rename = "POST_/functions/inventions/state")]
    PostFunctionsInventionsState,
    #[serde(rename = "POST_/functions/profiles/compute")]
    PostFunctionsProfilesCompute,
    #[serde(rename = "POST_/auth/keys")]
    PostAuthKeys,
    #[serde(rename = "POST_/auth/keys/openrouter")]
    PostAuthKeysOpenrouter,
    #[serde(rename = "DELETE_/auth/keys")]
    DeleteAuthKeys,
    #[serde(rename = "DELETE_/auth/keys/openrouter")]
    DeleteAuthKeysOpenrouter,
    #[serde(rename = "GET_/auth/keys")]
    GetAuthKeys,
    #[serde(rename = "GET_/auth/keys/openrouter")]
    GetAuthKeysOpenrouter,
    #[serde(rename = "GET_/auth/credits")]
    GetAuthCredits,
    #[serde(rename = "POST_/swarms/list")]
    PostSwarmsList,
    #[serde(rename = "POST_/swarms")]
    PostSwarms,
    #[serde(rename = "POST_/swarms/usage")]
    PostSwarmsUsage,
    #[serde(rename = "POST_/agents/list")]
    PostAgentsList,
    #[serde(rename = "POST_/agents")]
    PostAgents,
    #[serde(rename = "POST_/agents/usage")]
    PostAgentsUsage,
    #[serde(rename = "POST_/error")]
    PostError,
    #[serde(rename = "POST_/laboratories/executions")]
    PostLaboratoriesExecutions,
}

/// HTTP method an [`ApiCallSubType`] maps to. Mirrors the methods
/// `objectiveai-api`'s router uses (`POST`, `GET`, `DELETE`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
#[schemars(rename = "viewer.HttpMethod")]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

impl HttpMethod {
    /// Uppercase method name as it appears in the
    /// [`ApiCallSubType`] serde rename and HTTP wire format.
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Delete => "DELETE",
        }
    }
}

impl ApiCallSubType {
    /// HTTP method the viewer host should dispatch this sub-type with.
    pub fn method(&self) -> HttpMethod {
        match self {
            ApiCallSubType::GetAuthKeys
            | ApiCallSubType::GetAuthKeysOpenrouter
            | ApiCallSubType::GetAuthCredits => HttpMethod::Get,
            ApiCallSubType::DeleteAuthKeys
            | ApiCallSubType::DeleteAuthKeysOpenrouter => HttpMethod::Delete,
            _ => HttpMethod::Post,
        }
    }

    /// URL path this sub-type maps to on the `objectiveai-api`
    /// server. Always begins with a leading `/`.
    pub fn path(&self) -> &'static str {
        match self {
            ApiCallSubType::PostAgentCompletions => "/agent/completions",
            ApiCallSubType::PostVectorCompletions => "/vector/completions",
            ApiCallSubType::PostVectorCompletionsVotes => "/vector/completions/votes",
            ApiCallSubType::PostVectorCompletionsCache => "/vector/completions/cache",
            ApiCallSubType::PostFunctionsList => "/functions/list",
            ApiCallSubType::PostFunctions => "/functions",
            ApiCallSubType::PostFunctionsUsage => "/functions/usage",
            ApiCallSubType::PostFunctionsExecutions => "/functions/executions",
            ApiCallSubType::PostFunctionsProfilesList => "/functions/profiles/list",
            ApiCallSubType::PostFunctionsProfiles => "/functions/profiles",
            ApiCallSubType::PostFunctionsProfilesUsage => "/functions/profiles/usage",
            ApiCallSubType::PostFunctionsProfilesPairsList => "/functions/profiles/pairs/list",
            ApiCallSubType::PostFunctionsProfilesPairsUsage => "/functions/profiles/pairs/usage",
            ApiCallSubType::PostFunctionsInventions => "/functions/inventions",
            ApiCallSubType::PostFunctionsInventionsRecursive => "/functions/inventions/recursive",
            ApiCallSubType::PostFunctionsInventionsPromptsList => "/functions/inventions/prompts/list",
            ApiCallSubType::PostFunctionsInventionsPrompts => "/functions/inventions/prompts",
            ApiCallSubType::PostFunctionsInventionsPromptsUsage => "/functions/inventions/prompts/usage",
            ApiCallSubType::PostFunctionsInventionsState => "/functions/inventions/state",
            ApiCallSubType::PostFunctionsProfilesCompute => "/functions/profiles/compute",
            ApiCallSubType::PostAuthKeys => "/auth/keys",
            ApiCallSubType::PostAuthKeysOpenrouter => "/auth/keys/openrouter",
            ApiCallSubType::DeleteAuthKeys => "/auth/keys",
            ApiCallSubType::DeleteAuthKeysOpenrouter => "/auth/keys/openrouter",
            ApiCallSubType::GetAuthKeys => "/auth/keys",
            ApiCallSubType::GetAuthKeysOpenrouter => "/auth/keys/openrouter",
            ApiCallSubType::GetAuthCredits => "/auth/credits",
            ApiCallSubType::PostSwarmsList => "/swarms/list",
            ApiCallSubType::PostSwarms => "/swarms",
            ApiCallSubType::PostSwarmsUsage => "/swarms/usage",
            ApiCallSubType::PostAgentsList => "/agents/list",
            ApiCallSubType::PostAgents => "/agents",
            ApiCallSubType::PostAgentsUsage => "/agents/usage",
            ApiCallSubType::PostError => "/error",
            ApiCallSubType::PostLaboratoriesExecutions => "/laboratories/executions",
        }
    }
}

/// Wire-format envelope for each value the viewer host emits as
/// [`Event::ApiCall.value`](super::Event::ApiCall) while servicing one
/// `api-call-invoke` request.
///
/// Mirrors the cli's [`Output<T>`](crate::cli::output::Output)
/// envelope shape so iframe consumers can apply the same JSONL state
/// machine to both:
///
/// 1. `Begin` — exactly one, emitted before any data.
/// 2. `Chunk { chunk }` — one per SSE event for streaming endpoints,
///    or one total for unary endpoints (carrying the parsed response
///    body).
/// 3. `Error { error }` — only on dispatch failure; replaces any
///    further `Chunk`s for that invocation.
/// 4. `End` — exactly one, emitted last; signals the AsyncIterable on
///    the JS side to terminate.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "viewer.ApiCallEnvelope")]
pub enum ApiCallEnvelope {
    #[schemars(title = "Begin")]
    Begin,
    #[schemars(title = "Chunk")]
    Chunk { chunk: serde_json::Value },
    #[schemars(title = "Error")]
    Error { error: serde_json::Value },
    #[schemars(title = "End")]
    End,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sub_type_roundtrip_with_underscore_separator() {
        let s = ApiCallSubType::PostAgentCompletions;
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json, json!("POST_/agent/completions"));
        let back: ApiCallSubType = serde_json::from_value(json).unwrap();
        assert_eq!(back, ApiCallSubType::PostAgentCompletions);
    }

    #[test]
    fn auth_keys_disambiguated_by_method() {
        let post = ApiCallSubType::PostAuthKeys;
        let del = ApiCallSubType::DeleteAuthKeys;
        let get = ApiCallSubType::GetAuthKeys;
        assert_eq!(post.path(), "/auth/keys");
        assert_eq!(del.path(), "/auth/keys");
        assert_eq!(get.path(), "/auth/keys");
        assert_eq!(post.method(), HttpMethod::Post);
        assert_eq!(del.method(), HttpMethod::Delete);
        assert_eq!(get.method(), HttpMethod::Get);
        assert_eq!(
            serde_json::to_value(&post).unwrap(),
            json!("POST_/auth/keys")
        );
        assert_eq!(
            serde_json::to_value(&del).unwrap(),
            json!("DELETE_/auth/keys")
        );
        assert_eq!(serde_json::to_value(&get).unwrap(), json!("GET_/auth/keys"));
    }

    #[test]
    fn envelope_serializes_as_tagged_enum() {
        let begin = serde_json::to_value(ApiCallEnvelope::Begin).unwrap();
        assert_eq!(begin, json!({"type": "begin"}));

        let chunk = serde_json::to_value(ApiCallEnvelope::Chunk {
            chunk: json!({"x": 1}),
        })
        .unwrap();
        assert_eq!(chunk, json!({"type": "chunk", "chunk": {"x": 1}}));

        let end = serde_json::to_value(ApiCallEnvelope::End).unwrap();
        assert_eq!(end, json!({"type": "end"}));

        let err = serde_json::to_value(ApiCallEnvelope::Error {
            error: json!({"message": "boom"}),
        })
        .unwrap();
        assert_eq!(
            err,
            json!({"type": "error", "error": {"message": "boom"}})
        );
    }

    #[test]
    fn method_str_matches_serde_rename_prefix() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
        for variant in [
            ApiCallSubType::PostAgentCompletions,
            ApiCallSubType::GetAuthKeys,
            ApiCallSubType::DeleteAuthKeysOpenrouter,
        ] {
            let rename = serde_json::to_string(&variant).unwrap();
            // rename string is `"METHOD_/path"`; the first quote-stripped
            // prefix up to `_` is the method's as_str().
            let prefix = rename.trim_matches('"').split('_').next().unwrap();
            assert_eq!(prefix, variant.method().as_str());
        }
    }
}
