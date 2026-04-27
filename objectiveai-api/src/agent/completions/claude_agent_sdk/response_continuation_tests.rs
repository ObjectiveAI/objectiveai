use super::super::{ContinuationItem, UpstreamClient};
use super::State;

fn make_client() -> super::Client {
    super::Client::new(String::new(), true, 0, 180, 1)
}

#[test]
fn test_no_continuation_no_request_continuation() {
    let client = make_client();
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        None,
        &[],
        None,
    );
    assert_eq!(result, objectiveai::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        session_id: String::new(),
        mcp_sessions: indexmap::IndexMap::new(),
    });
}

#[test]
fn test_session_id_from_continuation_state() {
    let client = make_client();
    let continuation = vec![
        ContinuationItem::State(State {
            session_id: "internal-sess".into(),
            message_count: 1,
        }),
    ];
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        None,
        &[],
        Some(&continuation),
    );
    assert_eq!(result.session_id, "internal-sess");
}

#[test]
fn test_session_id_falls_back_to_request_continuation() {
    let client = make_client();
    let rc = objectiveai::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        session_id: "req-sess-123".into(),
        mcp_sessions: indexmap::IndexMap::new(),
    };
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        Some(&rc),
        &[],
        None,
    );
    assert_eq!(result.session_id, "req-sess-123");
}

#[test]
fn test_internal_session_id_takes_precedence() {
    let client = make_client();
    let continuation = vec![
        ContinuationItem::State(State {
            session_id: "internal-sess".into(),
            message_count: 1,
        }),
    ];
    let rc = objectiveai::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        session_id: "req-sess-456".into(),
        mcp_sessions: indexmap::IndexMap::new(),
    };
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        Some(&rc),
        &[],
        Some(&continuation),
    );
    assert_eq!(result.session_id, "internal-sess");
}

#[test]
fn test_empty_internal_session_falls_back_to_request() {
    let client = make_client();
    let continuation = vec![
        ContinuationItem::State(State {
            session_id: String::new(),
            message_count: 0,
        }),
    ];
    let rc = objectiveai::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        session_id: "req-sess-fallback".into(),
        mcp_sessions: indexmap::IndexMap::new(),
    };
    let result = client.response_continuation(
        indexmap::IndexMap::new(),
        Some(&rc),
        &[],
        Some(&continuation),
    );
    assert_eq!(result.session_id, "req-sess-fallback");
}

#[test]
fn test_mcp_sessions_preserved() {
    let client = make_client();
    let mut mcp_sessions = indexmap::IndexMap::new();
    mcp_sessions.insert("http://mcp.example.com".into(), "sess-abc".into());
    let result = client.response_continuation(
        mcp_sessions.clone(),
        None,
        &[],
        None,
    );
    assert_eq!(result.mcp_sessions, mcp_sessions);
}
