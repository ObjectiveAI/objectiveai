//! E2E: the `/user` user-requests channel — `user request` broadcasts
//! to connected user streams (SSE), the first ACCEPTED reply wins,
//! pending requests replay to new connections, settled ones don't,
//! and the optional python validator gates replies.
//!
//! EVENT-DRIVEN throughout — no polling, no sleeps, no deadlines:
//! every wait rides the listener's change-counter watch receiver
//! (held across iterations, so an event landing between a check and
//! its await is never missed) or the daemon's `Live` caught-up
//! marker. A wedged run is bounded by the suite harness, not by
//! in-test timers that flake under load.

mod cli_test_util;

use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::CommandExecutor;
use objectiveai_sdk::cli::command::user::request::{
    Path as UserRequestPath, Request as UserRequestRequest, Response as UserRequestResponse,
};
use objectiveai_sdk::cli::user_listener::{UserEvent, UserListener, UserReplyOutcome};

/// Await `$cond` over `$listener`'s events: check, and if unmet,
/// await the held change receiver — race-free (see
/// `UserListener::changes`), no polling, no deadline.
macro_rules! wait_until {
    ($listener:expr, $cond:expr) => {{
        let mut rx = $listener.changes();
        loop {
            if $cond {
                break;
            }
            rx.changed().await.expect("listener pump gone");
        }
    }};
}

fn request(
    key: &str,
    details: serde_json::Value,
    validate_python: Option<&str>,
) -> UserRequestRequest {
    UserRequestRequest {
        path_type: UserRequestPath::UserRequest,
        key: key.to_string(),
        details,
        validate_python: validate_python.map(String::from),
        base: Default::default(),
    }
}

fn replier() -> AgentArguments {
    AgentArguments {
        agent_instance_hierarchy: Some("human-tester".to_string()),
        ..Default::default()
    }
}

/// Connect a listener and await its `Live` marker — the caught-up
/// point: everything pending at connect has been delivered.
async fn connect_live(addr: &str) -> UserListener {
    let live = std::sync::Arc::new(tokio::sync::Notify::new());
    let notify = live.clone();
    let listener = UserListener::new(addr.to_string())
        .on_event(move |event| {
            if matches!(event, UserEvent::Live) {
                notify.notify_one();
            }
        })
        .connect()
        .await
        .expect("connect /user");
    live.notified().await;
    listener
}

/// The full happy path + replay semantics + validator gating.
#[tokio::test(flavor = "multi_thread")]
async fn user_request_first_accepted_reply_wins() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;
    let state = cli_test_util::test_state_name();
    let addr = cli_test_util::daemon_address(&executor, &state).await;

    // A user stream connected BEFORE the request: sees it live.
    let listener = connect_live(&addr).await;
    assert!(listener.pending().await.is_empty(), "fresh channel is empty");

    // Fire the blocking command in the background.
    let exec2 = cli_test_util::executor().await;
    let ask = tokio::spawn(async move {
        exec2
            .execute_one::<_, UserRequestResponse>(
                request("ask", serde_json::json!({"q": "proceed?"}), None),
                None,
            )
            .await
    });

    // The pending request reaches the live stream.
    wait_until!(listener, {
        listener.pending().await.iter().any(|r| r.key == "ask")
    });
    let pending = listener.pending().await;
    let req = pending.iter().find(|r| r.key == "ask").expect("pending ask");
    assert_eq!(req.details, serde_json::json!({"q": "proceed?"}));
    // Non-plugin caller: no plugin identity.
    assert!(req.plugin_owner.is_none());

    // A SECOND stream connected mid-pending: its Live marker follows
    // the replay, so by now the request is already in its view.
    let late_listener = connect_live(&addr).await;
    assert!(
        late_listener.pending().await.iter().any(|r| r.key == "ask"),
        "pending request must replay to a late connection"
    );

    // Reply from the first stream — wins.
    let outcome = listener
        .reply(&req.id, &replier(), serde_json::json!("yes"))
        .await
        .expect("reply");
    assert!(matches!(outcome, UserReplyOutcome::Accepted), "{outcome:?}");

    // The command returns the winning reply + the replier identity.
    let response = ask.await.expect("join").expect("user request");
    assert_eq!(response.reply, serde_json::json!("yes"));
    assert_eq!(
        response.identity.agent_instance_hierarchy.as_deref(),
        Some("human-tester")
    );

    // Both streams learn the settlement (pending drains everywhere).
    wait_until!(listener, { listener.pending().await.is_empty() });
    wait_until!(late_listener, { late_listener.pending().await.is_empty() });

    // A THIRD stream connected AFTER settlement: its caught-up point
    // proves absence — settled requests are never replayed.
    let after_listener = connect_live(&addr).await;
    assert!(after_listener.pending().await.is_empty());

    // A late reply to the settled id is rejected as unknown/settled.
    let late = listener
        .reply(&req.id, &replier(), serde_json::json!("me too"))
        .await
        .expect("late reply");
    assert!(
        matches!(late, UserReplyOutcome::NotFound | UserReplyOutcome::Settled),
        "{late:?}"
    );

    // ── Validator: only `True` accepts ───────────────────────────
    let exec3 = cli_test_util::executor().await;
    let gated = tokio::spawn(async move {
        exec3
            .execute_one::<_, UserRequestResponse>(
                request(
                    "gated",
                    serde_json::json!({"q": "yes or no"}),
                    Some("input['reply'] == 'yes'"),
                ),
                None,
            )
            .await
    });
    wait_until!(listener, {
        listener.pending().await.iter().any(|r| r.key == "gated")
    });
    let gated_id = listener
        .pending()
        .await
        .iter()
        .find(|r| r.key == "gated")
        .expect("gated pending")
        .id
        .clone();
    let rejected = listener
        .reply(&gated_id, &replier(), serde_json::json!("no"))
        .await
        .expect("rejected reply");
    assert!(matches!(rejected, UserReplyOutcome::Rejected { .. }), "{rejected:?}");
    // Still pending — the rejection left it open.
    assert!(listener.pending().await.iter().any(|r| r.id == gated_id));
    let accepted = listener
        .reply(&gated_id, &replier(), serde_json::json!("yes"))
        .await
        .expect("accepted reply");
    assert!(matches!(accepted, UserReplyOutcome::Accepted), "{accepted:?}");
    let gated_response = gated.await.expect("join").expect("gated request");
    assert_eq!(gated_response.reply, serde_json::json!("yes"));
}
