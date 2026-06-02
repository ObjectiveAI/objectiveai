//! Round-trip tests for every [`NotificationValue`] variant plus
//! [`Output::Error`] and the [`Handle`] emit paths.
//!
//! Each per-variant test builds a representative value, serializes it
//! through `Output::Notification`, parses the JSON back, and asserts
//! the deserialized value equals the original via `PartialEq`.

use std::sync::Arc;

use super::*;
use crate::cli::output::notification::{SkipReason, Updater};
use serde_json::json;
use tokio::sync::Mutex;

fn roundtrip(out: &Output) -> Output {
    let s = serde_json::to_string(out).expect("Output serializes");
    serde_json::from_str(&s).expect("Output deserializes")
}

fn notif(value: NotificationValue) -> Output {
    Output::Notification(Notification { value })
}

fn assert_roundtrip_eq(out: Output) {
    let back = roundtrip(&out);
    assert_eq!(out, back, "round-trip changed shape");
}

#[tokio::test]
async fn emit_via_stdout_handle() {
    // Smoke test that the default Stdout-destination handle routes
    // emit() without panicking. We can't intercept stdout from a unit
    // test, so just confirm the call completes.
    notif(NotificationValue::Typed(TypedNotificationValue::Ok(OK)))
        .emit(&Handle::stdout())
        .await;
}

#[tokio::test]
async fn emit_via_collect_handle_appends_to_vec() {
    let vec = Arc::new(Mutex::new(Vec::new()));
    let handle = Handle::from(HandleDestination::Collect(vec.clone()));

    notif(NotificationValue::Typed(TypedNotificationValue::Ok(OK)))
        .emit(&handle)
        .await;
    Output::Error(Error {
        r#type: ErrorType::Error,
        level: Level::Warn,
        fatal: false,
        message: "heads up".into(),
        agent_instance_hierarchy: None,
    })
    .emit(&handle)
    .await;

    let snapshot = vec.lock().await;
    assert_eq!(snapshot.len(), 2);

    let first = serde_json::to_value(&snapshot[0]).unwrap();
    assert_eq!(first["type"], "ok");
    assert_eq!(first["ok"], true);

    let second = serde_json::to_value(&snapshot[1]).unwrap();
    assert_eq!(second["type"], "error");
    assert_eq!(second["level"], "warn");
    assert_eq!(second["fatal"], false);
    assert_eq!(second["message"], "heads up");
}

#[test]
fn error_fatal_roundtrip() {
    let out = Output::Error(Error {
        r#type: ErrorType::Error,
        level: Level::Error,
        fatal: true,
        message: "favorite not found: foo".into(),
        agent_instance_hierarchy: None,
    });
    assert_roundtrip_eq(out);
}

#[test]
fn error_non_fatal_warn_roundtrip() {
    let out = Output::Error(Error {
        r#type: ErrorType::Error,
        level: Level::Warn,
        fatal: false,
        message: json!({"code": "x", "detail": [1, 2, 3]}),
        agent_instance_hierarchy: Some("cli".to_string()),
    });
    assert_roundtrip_eq(out);
}

// === Per-variant NotificationValue round-trip tests ===

#[test]
fn nv_active_agent_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::ActiveAgent(ActiveAgent {
            agent_id: "child-1".into(),
            last_log: 1_700_000_000,
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_agent_items_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::AgentItems(AgentItems {
            agent_id: "agent-1".into(),
            items: vec![],
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_inactive_roundtrip() {
    let out =
        notif(NotificationValue::Typed(TypedNotificationValue::Inactive(
            crate::cli::output::notification::agents::Inactive {
                agent_id: "agent-1".into(),
            },
        )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_spawned_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Spawned(
        Spawned {
            agent_id: "spawn-xyz".into(),
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_me_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Me(Me {
        agent_instance_hierarchy: "agent-xyz".into(),
    })));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_mcp_roundtrip() {
    let out =
        notif(NotificationValue::Typed(TypedNotificationValue::Mcp(Mcp {
            url: "http://127.0.0.1:9876".into(),
        })));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_message_delivered_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::MessageDelivered(MessageDelivered {
            agent_id: "cli/foo-123".into(),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_message_queued_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::MessageQueued(MessageQueued {
            agent_id: "cli/foo-123".into(),
            response_id: "resp-abc".into(),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_detached_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::Detached(Detached { pid: 12345 }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_command_complete_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::CommandComplete(CommandComplete { exit_code: 0 }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_inventions_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::Inventions(Inventions {
            inventions: vec![InventionResultItem {
                name: "alpha_scalar_leaf".into(),
                path: None,
            }],
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_cleared_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Cleared(
        Cleared { cleared: 7 },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_help_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Help(
        Help {
            help: "Usage: objectiveai [OPTIONS]".into(),
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_installed_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::Installed(Installed { installed: true }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_instructions_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::Instructions(Instructions {
            instructions: "# Setup\n\n…".into(),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_jq_results_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::JqResults(JqResults {
            jq: json!([{"a": 1}, {"b": 2}]),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_log_content_json_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::LogContent(LogContent::Json {
            content: json!({"completion": {"id": "abc"}}),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_log_content_text_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::LogContent(LogContent::Text {
            text: "plain string".into(),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_log_content_image_roundtrip() {
    use crate::agent::completions::message::ImageUrl;
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::LogContent(LogContent::Image {
            image_url: ImageUrl {
                url: "data:image/png;base64,iVBORw0KGgo".into(),
                detail: None,
            },
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_log_content_audio_roundtrip() {
    use crate::agent::completions::message::InputAudio;
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::LogContent(LogContent::Audio {
            input_audio: InputAudio {
                data: "SUQzBAA".into(),
                format: "audio/mpeg".into(),
            },
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_log_content_video_roundtrip() {
    use crate::agent::completions::message::VideoUrl;
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::LogContent(LogContent::Video {
            video_url: VideoUrl {
                url: "data:video/mp4;base64,AAAA".into(),
            },
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_log_content_file_roundtrip() {
    use crate::agent::completions::message::File;
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::LogContent(LogContent::File {
            file: File {
                file_data: Some("JVBERi0".into()),
                filename: Some("report.pdf".into()),
                file_id: None,
                file_url: None,
            },
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_log_stream_ready_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::LogStreamReady(LogStreamReady {
            log_stream_ready: "abc-123".into(),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_ok_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Ok(OK)));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_published_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::Published(Published {
            sha: "deadbeef".into(),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_schema_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Schema(
        Schema {
            schema: json!({"$schema": "...", "type": "object"}),
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_schemas_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Schemas(
        Schemas {
            schemas: vec!["Foo".into(), "Bar".into()],
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_tool_line_stdout_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::ToolLine(ToolLine {
            line: "hello".into(),
            stdout: Some(true),
            stderr: None,
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_tool_line_stderr_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::ToolLine(ToolLine {
            line: "oops".into(),
            stdout: None,
            stderr: Some(true),
        }),
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugins_empty_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Plugins(
        Plugins { plugins: vec![] },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugin_none_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Plugin(
        Plugin { plugin: None },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugin_notification_roundtrip_object() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::PluginNotification {
            value: serde_json::json!({"hello": "world", "count": 3}),
        },
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugin_notification_roundtrip_string() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::PluginNotification {
            value: serde_json::Value::String("plain text payload".into()),
        },
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugin_notification_roundtrip_bool() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::PluginNotification {
            value: serde_json::Value::Bool(true),
        },
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugin_notification_roundtrip_number() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::PluginNotification {
            value: serde_json::json!(42),
        },
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugin_notification_roundtrip_array() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::PluginNotification {
            value: serde_json::json!([1, "two", false, null]),
        },
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_plugin_notification_roundtrip_null() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::PluginNotification {
            value: serde_json::Value::Null,
        },
    ));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_tools_empty_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Tools(
        Tools { tools: vec![] },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_tool_none_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Tool(
        Tool { tool: None },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_updater_checking_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Updater(
        Updater::Checking {
            asset_name: "objectiveai-x86_64-linux".into(),
            current_version: "1.0.0".into(),
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_updater_skipped_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Updater(
        Updater::Skipped {
            reason: SkipReason::DevTree,
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_updater_up_to_date_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Updater(
        Updater::UpToDate {
            current_version: "1.0.0".into(),
            remote_version: "1.0.0".into(),
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_updater_found_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Updater(
        Updater::Found {
            current_version: "1.0.0".into(),
            remote_version: "1.1.0".into(),
            asset_name: "asset.tar.gz".into(),
            url: "https://example.com/asset.tar.gz".into(),
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_updater_installed_roundtrip() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Updater(
        Updater::Installed {
            current_version: "1.0.0".into(),
            remote_version: "1.1.0".into(),
        },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_viewer_send_result_roundtrip() {
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::ViewerSendResult(ViewerSendResult {
            status: 200,
            body: json!({"ok": true}),
        }),
    ));
    assert_roundtrip_eq(out);
}

// === Fixtures for wrappers around deep API response types. Built
// via Rust value construction (mock-backed default builders) rather
// than hand-rolled JSON so the round-trip catches schema drift, not
// fixture typos. ===

fn mock_remote_path() -> crate::RemotePath {
    crate::RemotePath::Mock {
        name: "demo".to_string(),
    }
}

fn mock_agent_with_fallbacks() -> crate::agent::RemoteAgentWithFallbacks {
    let base = crate::agent::mock::AgentBase::default();
    let inner = crate::agent::InlineAgentBaseWithFallbacks {
        inner: crate::agent::InlineAgentBase::Mock(base),
        fallbacks: None,
    };
    let remote_base = crate::agent::RemoteAgentBaseWithFallbacks {
        description: "demo agent".to_string(),
        inner,
    };
    remote_base.convert().expect("mock agent converts")
}

#[test]
fn nv_agent_roundtrip() {
    let response = crate::agent::response::GetAgentResponse {
        path: mock_remote_path(),
        inner: mock_agent_with_fallbacks(),
    };
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Agent(
        crate::cli::output::notification::Agent { agent: response },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_swarm_roundtrip() {
    // RemoteSwarmBase::convert with one mock agent slot.
    let agent_slot =
        crate::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
            count: 1,
            inner:
                crate::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                    crate::agent::InlineAgentBaseWithFallbacks {
                        inner: crate::agent::InlineAgentBase::Mock(
                            crate::agent::mock::AgentBase::default(),
                        ),
                        fallbacks: None,
                    },
                ),
        };
    let inline_base = crate::swarm::InlineSwarmBase {
        agents: vec![agent_slot],
        weights: None,
    };
    let remote_base = crate::swarm::RemoteSwarmBase {
        description: "demo swarm".to_string(),
        inner: inline_base,
    };
    let remote_swarm = remote_base.convert(None).expect("swarm converts");
    let response = crate::swarm::response::GetSwarmResponse {
        path: mock_remote_path(),
        inner: remote_swarm,
    };
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Swarm(
        crate::cli::output::notification::Swarm { swarm: response },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_execution_roundtrip() {
    // TaskOutputOwned::Scalar is the simplest output variant.
    let out =
        notif(NotificationValue::Typed(TypedNotificationValue::Execution(
            crate::cli::output::notification::Execution {
                execution: crate::cli::output::notification::ExecutionResult {
                    output:
                        crate::functions::expression::TaskOutputOwned::Scalar(
                            rust_decimal::Decimal::new(5, 1), // 0.5
                        ),
                },
            },
        )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_laboratory_roundtrip() {
    // LabResultItem holds an agent + optional score. Use the Remote
    // variant pointing at a mock path — sidesteps inline-agent
    // construction.
    let item = crate::cli::output::notification::LabResultItem {
        agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(
            crate::RemotePathCommitOptional::Mock { name: "demo".to_string() },
        ),
        score: Some(0.75),
    };
    let out = notif(NotificationValue::Typed(
        TypedNotificationValue::Laboratory(
            crate::cli::output::notification::Laboratory {
                laboratory: vec![item],
            },
        ),
    ));
    assert_roundtrip_eq(out);
}

/// Deterministic Arbitrary-based builder for types whose direct
/// construction would require walking deep flatten chains. Seed bytes
/// are constant per-fixture so a regression surfaces as a stable diff
/// rather than a flaky one.
fn arb<'a, T: arbitrary::Arbitrary<'a>>(seed: &'a [u8]) -> T {
    let mut u = arbitrary::Unstructured::new(seed);
    T::arbitrary(&mut u).expect("arbitrary fixture")
}

/// Smallest valid RemoteFunction: a Standard Scalar with empty
/// any-of input schema and no tasks. Used by Function + Pair to
/// avoid Arbitrary's f32→f64 precision drift on number bounds.
fn minimal_remote_function() -> crate::functions::FullRemoteFunction {
    crate::functions::FullRemoteFunction::Standard(
        crate::functions::RemoteFunction::Scalar {
            description: "demo function".to_string(),
            input_schema: crate::functions::expression::InputSchema::AnyOf(
                crate::functions::expression::AnyOfInputSchema {
                    any_of: vec![],
                },
            ),
            tasks: vec![],
        },
    )
}

#[test]
fn nv_function_roundtrip() {
    let response = crate::functions::response::GetFunctionResponse {
        path: mock_remote_path(),
        inner: minimal_remote_function(),
    };
    let out =
        notif(NotificationValue::Typed(TypedNotificationValue::Function(
            crate::cli::output::notification::Function { function: response },
        )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_profile_roundtrip() {
    // RemoteProfile::Auto wraps a RemoteSwarmBase — minimal swarm
    // base with one mock agent slot.
    let agent_slot =
        crate::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
            count: 1,
            inner:
                crate::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                    crate::agent::InlineAgentBaseWithFallbacks {
                        inner: crate::agent::InlineAgentBase::Mock(
                            crate::agent::mock::AgentBase::default(),
                        ),
                        fallbacks: None,
                    },
                ),
        };
    let swarm_base = crate::swarm::RemoteSwarmBase {
        description: "demo profile swarm".to_string(),
        inner: crate::swarm::InlineSwarmBase {
            agents: vec![agent_slot],
            weights: None,
        },
    };
    let inner = crate::functions::RemoteProfile::Auto(swarm_base);
    let response = crate::functions::profiles::response::GetProfileResponse {
        path: mock_remote_path(),
        inner,
    };
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Profile(
        crate::cli::output::notification::Profile { profile: response },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_pair_roundtrip() {
    let function = crate::functions::response::GetFunctionResponse {
        path: mock_remote_path(),
        inner: minimal_remote_function(),
    };
    let agent_slot =
        crate::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
            count: 1,
            inner:
                crate::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                    crate::agent::InlineAgentBaseWithFallbacks {
                        inner: crate::agent::InlineAgentBase::Mock(
                            crate::agent::mock::AgentBase::default(),
                        ),
                        fallbacks: None,
                    },
                ),
        };
    let swarm_base = crate::swarm::RemoteSwarmBase {
        description: "demo pair swarm".to_string(),
        inner: crate::swarm::InlineSwarmBase {
            agents: vec![agent_slot],
            weights: None,
        },
    };
    let profile = crate::functions::profiles::response::GetProfileResponse {
        path: mock_remote_path(),
        inner: crate::functions::RemoteProfile::Auto(swarm_base),
    };
    let pair = crate::cli::output::notification::FunctionProfilePair {
        function,
        profile,
    };
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Pair(
        crate::cli::output::notification::Pair { pair },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_state_roundtrip() {
    // ParamsState::AlphaScalarLeaf wraps Params which has its own
    // `name` field — collides with `RemotePath::Mock.name` under
    // serde flatten. Use the Github RemotePath variant (no `name`)
    // for this fixture, and hand-build the inner state to skip
    // Arbitrary's f32→f64 precision drift on schema bounds.
    let leaf = crate::functions::inventions::state::AlphaScalarLeafState {
        params: crate::functions::inventions::state::Params {
            depth: 3,
            min_branch_width: 1,
            max_branch_width: 2,
            min_leaf_width: 1,
            max_leaf_width: 2,
            name: "demo".to_string(),
            spec: "demo spec".to_string(),
        },
        essay: None,
        input_schema: None,
        essay_tasks: None,
        tasks: None,
        tasks_length: None,
        description: None,
        readme: None,
        checker_seed: None,
    };
    let inner =
        crate::functions::inventions::state::ParamsState::AlphaScalarLeaf(leaf);
    let response = crate::functions::inventions::state::response::GetFunctionInventionStateResponse {
        path: crate::RemotePath::Github {
            owner: "demo".to_string(),
            repository: "inv".to_string(),
            commit: "0".repeat(40),
        },
        inner,
    };
    let out = notif(NotificationValue::Typed(TypedNotificationValue::State(
        crate::cli::output::notification::State { state: response },
    )));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_other_items_roundtrip() {
    // The catch-all: an `Items<T>` payload routes through Other.
    let payload = Items {
        items: vec!["a".to_string(), "b".to_string()],
    };
    let out = notif(NotificationValue::other(&payload));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_other_value_roundtrip() {
    // The catch-all: a `Value<V>` payload routes through Other.
    let payload = Value {
        value: vec![1u32, 2, 3],
    };
    let out = notif(NotificationValue::other(&payload));
    assert_roundtrip_eq(out);
}

#[test]
fn nv_other_raw_object_roundtrip() {
    let payload =
        json!({"arbitrary": {"nested": [1, 2, 3]}, "kind_hint": null});
    let out = notif(NotificationValue::other(&payload));
    assert_roundtrip_eq(out);
}

#[test]
fn full_envelope_with_agent_instance_hierarchy_roundtrip() {
    let out = Output::Notification(Notification {
        value: NotificationValue::Typed(TypedNotificationValue::Spawned(
            Spawned {
                agent_id: "x".into(),
            },
        )),
    });
    assert_roundtrip_eq(out);
}

#[test]
fn other_keys_flatten_at_top_level() {
    // Sanity check: the catch-all variant's map keys land directly
    // at the Notification level — no `type`/`kind` envelope, no
    // `value` wrapper. Other is the untagged half of NotificationValue.
    let out = notif(NotificationValue::other(&json!({"foo": 1, "bar": "baz"})));
    let v = serde_json::to_value(&out).unwrap();
    assert!(v.get("type").is_none(), "Other has no `type` tag");
    assert!(v.get("value").is_none(), "no `value` wrapper");
    assert_eq!(v["foo"], 1);
    assert_eq!(v["bar"], "baz");
}

#[test]
fn typed_variant_carries_type_discriminator() {
    let out = notif(NotificationValue::Typed(TypedNotificationValue::Spawned(
        Spawned {
            agent_id: "abc".into(),
        },
    )));
    let v = serde_json::to_value(&out).unwrap();
    assert_eq!(v["type"], "spawned");
    assert_eq!(v["agent_id"], "abc");
    assert!(v.get("value").is_none(), "no `value` wrapper");
}
