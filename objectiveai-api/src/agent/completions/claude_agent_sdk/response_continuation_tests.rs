use super::super::{ContinuationItem, UpstreamClient};
use super::State;

fn make_client() -> super::Client {
    // objectiveai_dir is unused by response_continuation (no runner is
    // spawned in these pure-logic tests), so a dummy path is fine.
    super::Client::new(String::new(), true, 0, 180, 1, std::path::PathBuf::new())
}

#[test]
fn test_no_continuation_no_request_continuation() {
    let client = make_client();
    let result = client.response_continuation(
        None,
        &[],
        None,
        ""
    );
    assert_eq!(result, objectiveai_sdk::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        agent_instance_hierarchy: String::new(),
        session_id: String::new(),
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
        None,
        &[],
        Some(&continuation),
        ""
    );
    assert_eq!(result.session_id, "internal-sess");
}

#[test]
fn test_session_id_falls_back_to_request_continuation() {
    let client = make_client();
    let rc = objectiveai_sdk::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        agent_instance_hierarchy: String::new(),
        session_id: "req-sess-123".into(),
    };
    let result = client.response_continuation(
        Some(&rc),
        &[],
        None,
        ""
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
    let rc = objectiveai_sdk::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        agent_instance_hierarchy: String::new(),
        session_id: "req-sess-456".into(),
    };
    let result = client.response_continuation(
        Some(&rc),
        &[],
        Some(&continuation),
        ""
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
    let rc = objectiveai_sdk::agent::claude_agent_sdk::Continuation {
        upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
        agent_instance_hierarchy: String::new(),
        session_id: "req-sess-fallback".into(),
    };
    let result = client.response_continuation(
        Some(&rc),
        &[],
        Some(&continuation),
        ""
    );
    assert_eq!(result.session_id, "req-sess-fallback");
}
