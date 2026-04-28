//! Round-trip tests for the Codex SDK type definitions in
//! `objectiveai_api::agent::completions::codex_sdk`. Lives as an integration
//! test so it doesn't share a build target with the lib's `#[cfg(test)]`
//! suite — those break independently when in-progress refactors land.

use objectiveai_api::agent::completions::codex_sdk::*;
use serde_json::json;

// ---------------------------------------------------------------------------
// ThreadEvent — every variant from `_EVENT_MODELS` in parsing.py
// ---------------------------------------------------------------------------

#[test]
fn thread_started_event_round_trips() {
    let wire = json!({"type": "thread.started", "thread_id": "thr-abc"});
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(
        &parsed,
        ThreadEvent::Known(KnownThreadEvent::ThreadStarted(e)) if e.thread_id == "thr-abc"
    ));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn turn_started_event_round_trips() {
    let wire = json!({"type": "turn.started"});
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(parsed, ThreadEvent::Known(KnownThreadEvent::TurnStarted(_))));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn turn_completed_event_round_trips() {
    let wire = json!({
        "type": "turn.completed",
        "usage": {
            "input_tokens": 100,
            "cached_input_tokens": 20,
            "output_tokens": 50,
        },
    });
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    let ThreadEvent::Known(KnownThreadEvent::TurnCompleted(e)) = &parsed else { panic!("wrong variant") };
    assert_eq!(e.usage.input_tokens, 100);
    assert_eq!(e.usage.cached_input_tokens, 20);
    assert_eq!(e.usage.output_tokens, 50);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn turn_failed_event_round_trips() {
    let wire = json!({
        "type": "turn.failed",
        "error": {"message": "boom"},
    });
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    let ThreadEvent::Known(KnownThreadEvent::TurnFailed(e)) = &parsed else { panic!("wrong variant") };
    assert_eq!(e.error.message, "boom");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn item_started_event_with_agent_message_round_trips() {
    let wire = json!({
        "type": "item.started",
        "item": {
            "type": "agent_message",
            "id": "item-1",
            "text": "hello",
        },
    });
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    let ThreadEvent::Known(KnownThreadEvent::ItemStarted(e)) = &parsed else { panic!("wrong variant") };
    let ThreadItem::Known(KnownThreadItem::AgentMessage(item)) = &e.item else { panic!("wrong item variant") };
    assert_eq!(item.id, "item-1");
    assert_eq!(item.text, "hello");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn item_updated_event_with_command_execution_round_trips() {
    let wire = json!({
        "type": "item.updated",
        "item": {
            "type": "command_execution",
            "id": "cmd-1",
            "command": "ls -la",
            "aggregated_output": "total 0\n",
            "status": "in_progress",
        },
    });
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(parsed, ThreadEvent::Known(KnownThreadEvent::ItemUpdated(_))));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn item_completed_event_with_command_execution_with_exit_code_round_trips() {
    let wire = json!({
        "type": "item.completed",
        "item": {
            "type": "command_execution",
            "id": "cmd-1",
            "command": "ls -la",
            "aggregated_output": "total 0\n",
            "exit_code": 0,
            "status": "completed",
        },
    });
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    let ThreadEvent::Known(KnownThreadEvent::ItemCompleted(e)) = &parsed else { panic!("wrong variant") };
    let ThreadItem::Known(KnownThreadItem::CommandExecution(item)) = &e.item else { panic!("wrong item variant") };
    assert_eq!(item.exit_code, Some(0));
    assert_eq!(item.status, CommandExecutionStatus::Completed);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn thread_error_event_round_trips() {
    let wire = json!({"type": "error", "message": "something broke"});
    let parsed: ThreadEvent = serde_json::from_value(wire.clone()).unwrap();
    let ThreadEvent::Known(KnownThreadEvent::Error(e)) = &parsed else { panic!("wrong variant") };
    assert_eq!(e.message, "something broke");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn unknown_event_type_falls_back() {
    let wire = json!({
        "type": "thread.something_new",
        "thread_id": "x",
        "extra_field": 42,
    });
    let parsed: ThreadEvent = serde_json::from_value(wire).unwrap();
    let ThreadEvent::Unknown(unknown) = &parsed else { panic!("expected Unknown") };
    assert_eq!(unknown.r#type, "thread.something_new");
    // Re-serialization keeps only the discriminator — extras are discarded.
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        json!({"type": "thread.something_new"}),
    );
}

#[test]
fn unknown_item_type_falls_back() {
    let wire = json!({
        "type": "future_item",
        "id": "i-1",
        "novel_field": "value",
    });
    let parsed: ThreadItem = serde_json::from_value(wire).unwrap();
    let ThreadItem::Unknown(unknown) = &parsed else { panic!("expected Unknown") };
    assert_eq!(unknown.r#type, "future_item");
    assert_eq!(unknown.id, "i-1");
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        json!({"id": "i-1", "type": "future_item"}),
    );
}

#[test]
fn item_event_with_unknown_inner_item_falls_back() {
    let wire = json!({
        "type": "item.completed",
        "item": {
            "type": "future_item",
            "id": "i-2",
            "weird": true,
        },
    });
    let parsed: ThreadEvent = serde_json::from_value(wire).unwrap();
    let ThreadEvent::Known(KnownThreadEvent::ItemCompleted(e)) = &parsed else { panic!("wrong variant") };
    let ThreadItem::Unknown(item) = &e.item else { panic!("expected Unknown inner item") };
    assert_eq!(item.r#type, "future_item");
    assert_eq!(item.id, "i-2");
}

// ---------------------------------------------------------------------------
// ThreadItem — every variant from `_ITEM_MODELS` in parsing.py
// ---------------------------------------------------------------------------

#[test]
fn agent_message_item_round_trips() {
    let wire = json!({"type": "agent_message", "id": "i", "text": "hi"});
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(parsed, ThreadItem::Known(KnownThreadItem::AgentMessage(_))));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn reasoning_item_round_trips() {
    let wire = json!({"type": "reasoning", "id": "r", "text": "thinking..."});
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(parsed, ThreadItem::Known(KnownThreadItem::Reasoning(_))));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn file_change_item_round_trips() {
    let wire = json!({
        "type": "file_change",
        "id": "fc",
        "changes": [
            {"path": "a.rs", "kind": "add"},
            {"path": "b.rs", "kind": "update"},
            {"path": "c.rs", "kind": "delete"},
        ],
        "status": "completed",
    });
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    let ThreadItem::Known(KnownThreadItem::FileChange(item)) = &parsed else { panic!("wrong variant") };
    assert_eq!(item.changes.len(), 3);
    assert_eq!(item.changes[0].kind, PatchChangeKind::Add);
    assert_eq!(item.changes[1].kind, PatchChangeKind::Update);
    assert_eq!(item.changes[2].kind, PatchChangeKind::Delete);
    assert_eq!(item.status, PatchApplyStatus::Completed);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn mcp_tool_call_item_with_arbitrary_arguments_round_trips() {
    let wire = json!({
        "type": "mcp_tool_call",
        "id": "tool-1",
        "server": "fs",
        "tool": "read_file",
        "arguments": {"path": "/tmp/x", "options": [1, 2, {"nested": true}]},
        "status": "in_progress",
    });
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    let ThreadItem::Known(KnownThreadItem::McpToolCall(item)) = &parsed else { panic!("wrong variant") };
    assert_eq!(item.server, "fs");
    assert_eq!(item.tool, "read_file");
    assert_eq!(item.arguments["path"], "/tmp/x");
    assert_eq!(item.arguments["options"][2]["nested"], true);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn mcp_tool_call_item_with_result_round_trips() {
    let wire = json!({
        "type": "mcp_tool_call",
        "id": "tool-2",
        "server": "fs",
        "tool": "read_file",
        "arguments": {},
        "result": {
            "content": [{"type": "text", "text": "file body"}],
            "structured_content": {"size": 9},
        },
        "status": "completed",
    });
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    let ThreadItem::Known(KnownThreadItem::McpToolCall(item)) = &parsed else { panic!("wrong variant") };
    let result = item.result.as_ref().unwrap();
    assert_eq!(result.content.len(), 1);
    assert_eq!(result.structured_content["size"], 9);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn web_search_item_round_trips() {
    let wire = json!({"type": "web_search", "id": "w", "query": "rustlang"});
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(parsed, ThreadItem::Known(KnownThreadItem::WebSearch(_))));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn todo_list_item_round_trips() {
    let wire = json!({
        "type": "todo_list",
        "id": "t",
        "items": [
            {"text": "step 1", "completed": true},
            {"text": "step 2", "completed": false},
        ],
    });
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    let ThreadItem::Known(KnownThreadItem::TodoList(item)) = &parsed else { panic!("wrong variant") };
    assert_eq!(item.items.len(), 2);
    assert!(item.items[0].completed);
    assert!(!item.items[1].completed);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn error_item_round_trips() {
    let wire = json!({"type": "error", "id": "e", "message": "nope"});
    let parsed: ThreadItem = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(parsed, ThreadItem::Known(KnownThreadItem::Error(_))));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

// ---------------------------------------------------------------------------
// Options — populate_by_name=True parity (snake_case AND camelCase accepted)
// ---------------------------------------------------------------------------

#[test]
fn thread_options_accepts_snake_case() {
    let wire = json!({
        "model": "gpt-5",
        "sandbox_mode": "workspace-write",
        "approval_policy": "never",
        "model_reasoning_effort": "high",
        "skip_git_repo_check": true,
    });
    let parsed: ThreadOptions = serde_json::from_value(wire).unwrap();
    assert_eq!(parsed.model.as_deref(), Some("gpt-5"));
    assert_eq!(parsed.sandbox_mode, Some(SandboxMode::WorkspaceWrite));
    assert_eq!(parsed.approval_policy, Some(ApprovalMode::Never));
    assert_eq!(parsed.model_reasoning_effort, Some(ModelReasoningEffort::High));
    assert_eq!(parsed.skip_git_repo_check, Some(true));
}

#[test]
fn thread_options_accepts_camel_case() {
    let wire = json!({
        "model": "gpt-5",
        "sandboxMode": "workspace-write",
        "approvalPolicy": "never",
        "modelReasoningEffort": "high",
        "skipGitRepoCheck": true,
    });
    let parsed: ThreadOptions = serde_json::from_value(wire).unwrap();
    assert_eq!(parsed.sandbox_mode, Some(SandboxMode::WorkspaceWrite));
    assert_eq!(parsed.approval_policy, Some(ApprovalMode::Never));
    assert_eq!(parsed.model_reasoning_effort, Some(ModelReasoningEffort::High));
    assert_eq!(parsed.skip_git_repo_check, Some(true));
}

#[test]
fn thread_options_serializes_snake_case() {
    let opts = ThreadOptions {
        model: Some("gpt-5".into()),
        sandbox_mode: Some(SandboxMode::ReadOnly),
        ..Default::default()
    };
    let wire = serde_json::to_value(&opts).unwrap();
    assert_eq!(wire["model"], "gpt-5");
    assert_eq!(wire["sandbox_mode"], "read-only");
    assert!(wire.get("sandboxMode").is_none());
}

#[test]
fn codex_options_accepts_both_cases() {
    let snake = json!({"codex_path_override": "/p", "base_url": "https://x", "api_key": "k"});
    let camel = json!({"codexPathOverride": "/p", "baseUrl": "https://x", "apiKey": "k"});
    let a: CodexOptions = serde_json::from_value(snake).unwrap();
    let b: CodexOptions = serde_json::from_value(camel).unwrap();
    assert_eq!(a, b);
}

#[test]
fn turn_options_carries_arbitrary_output_schema() {
    let wire = json!({
        "output_schema": {
            "type": "object",
            "properties": {"x": {"type": "integer"}},
            "required": ["x"],
        },
    });
    let parsed: TurnOptions = serde_json::from_value(wire.clone()).unwrap();
    let schema = parsed.output_schema.as_ref().unwrap();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["x"]["type"], "integer");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

// ---------------------------------------------------------------------------
// CodexExecArgs — flag construction must match _build_command_args ordering.
// Reference: openai_codex_sdk/exec.py:51-100.
// ---------------------------------------------------------------------------

#[test]
fn exec_args_minimal() {
    let args = CodexExecArgs {
        input: "prompt".into(),
        ..Default::default()
    };
    assert_eq!(
        args.to_command_args(),
        vec!["exec".to_string(), "--experimental-json".into()]
    );
}

#[test]
fn exec_args_full() {
    let args = CodexExecArgs {
        input: "prompt".into(),
        model: Some("gpt-5".into()),
        sandbox_mode: Some(SandboxMode::WorkspaceWrite),
        working_directory: Some("/work".into()),
        additional_directories: Some(vec!["/a".into(), "/b".into()]),
        skip_git_repo_check: Some(true),
        output_schema_file: Some("/tmp/schema.json".into()),
        model_reasoning_effort: Some(ModelReasoningEffort::High),
        network_access_enabled: Some(true),
        web_search_enabled: Some(false),
        approval_policy: Some(ApprovalMode::Never),
        images: Some(vec!["/img1.png".into(), "/img2.png".into()]),
        thread_id: Some("thr-1".into()),
        ..Default::default()
    };
    let expected: Vec<String> = [
        "exec",
        "--experimental-json",
        "--model",
        "gpt-5",
        "--sandbox",
        "workspace-write",
        "--cd",
        "/work",
        "--add-dir",
        "/a",
        "--add-dir",
        "/b",
        "--skip-git-repo-check",
        "--output-schema",
        "/tmp/schema.json",
        "--config",
        "model_reasoning_effort=\"high\"",
        "--config",
        "sandbox_workspace_write.network_access=true",
        "--config",
        "features.web_search_request=false",
        "--config",
        "approval_policy=\"never\"",
        "--image",
        "/img1.png",
        "--image",
        "/img2.png",
        "resume",
        "thr-1",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(args.to_command_args(), expected);
}

#[test]
fn exec_args_skip_flag_omitted_when_false() {
    let args = CodexExecArgs {
        input: "p".into(),
        skip_git_repo_check: Some(false),
        ..Default::default()
    };
    assert!(!args.to_command_args().iter().any(|a| a == "--skip-git-repo-check"));
}

#[test]
fn exec_args_env_layers_base_url_and_api_key() {
    use indexmap::IndexMap;
    let args = CodexExecArgs {
        input: "p".into(),
        base_url: Some("https://example.test".into()),
        api_key: Some("secret".into()),
        ..Default::default()
    };
    let mut base = IndexMap::new();
    base.insert("PATH".into(), "/usr/bin".into());
    let env = args.to_env(base);
    assert_eq!(env.get("PATH").map(String::as_str), Some("/usr/bin"));
    assert_eq!(env.get("OPENAI_BASE_URL").map(String::as_str), Some("https://example.test"));
    assert_eq!(env.get("CODEX_API_KEY").map(String::as_str), Some("secret"));
    assert_eq!(
        env.get("CODEX_INTERNAL_ORIGINATOR_OVERRIDE").map(String::as_str),
        Some("codex_sdk_rs"),
    );
}

#[test]
fn exec_args_env_does_not_clobber_existing_originator() {
    use indexmap::IndexMap;
    let args = CodexExecArgs { input: "p".into(), ..Default::default() };
    let mut base = IndexMap::new();
    base.insert("CODEX_INTERNAL_ORIGINATOR_OVERRIDE".into(), "test_harness".into());
    let env = args.to_env(base);
    assert_eq!(
        env.get("CODEX_INTERNAL_ORIGINATOR_OVERRIDE").map(String::as_str),
        Some("test_harness"),
        "caller-supplied originator must win over the SDK default",
    );
}

// ---------------------------------------------------------------------------
// Strict input parsing — TextInput / LocalImageInput have extra="forbid".
// ---------------------------------------------------------------------------

#[test]
fn text_input_rejects_unknown_fields() {
    let wire = json!({"type": "text", "text": "hi", "rogue": 1});
    let result: Result<UserInput, _> = serde_json::from_value(wire);
    assert!(result.is_err(), "unknown fields on TextInput must be rejected");
}

#[test]
fn local_image_input_rejects_unknown_fields() {
    let wire = json!({"type": "local_image", "path": "/tmp/x", "rogue": 1});
    let result: Result<UserInput, _> = serde_json::from_value(wire);
    assert!(result.is_err(), "unknown fields on LocalImageInput must be rejected");
}

// ---------------------------------------------------------------------------
// UserInput
// ---------------------------------------------------------------------------

#[test]
fn user_input_text_round_trips() {
    let wire = json!({"type": "text", "text": "hi"});
    let parsed: UserInput = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(&parsed, UserInput::Text(t) if t.text == "hi"));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn user_input_local_image_round_trips() {
    let wire = json!({"type": "local_image", "path": "/tmp/img.png"});
    let parsed: UserInput = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(&parsed, UserInput::LocalImage(i) if i.path == "/tmp/img.png"));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

// ---------------------------------------------------------------------------
// Turn — buffered run result
// ---------------------------------------------------------------------------

#[test]
fn turn_round_trips() {
    let wire = json!({
        "items": [
            {"type": "agent_message", "id": "m", "text": "answer"},
        ],
        "final_response": "answer",
        "usage": {"input_tokens": 1, "cached_input_tokens": 0, "output_tokens": 1},
    });
    let parsed: Turn = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed.final_response, "answer");
    assert_eq!(parsed.items.len(), 1);
    assert!(parsed.usage.is_some());
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

// ---------------------------------------------------------------------------
// Input — Python's `Union[str, List[Union[UserInput, Dict[str, Any]]]]`
// ---------------------------------------------------------------------------

#[test]
fn input_string_round_trips() {
    let wire = json!("hello");
    let parsed: Input = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(&parsed, Input::String(s) if s == "hello"));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn input_list_of_typed_user_inputs_round_trips() {
    let wire = json!([
        {"type": "text", "text": "first"},
        {"type": "local_image", "path": "/tmp/x.png"},
    ]);
    let parsed: Input = serde_json::from_value(wire.clone()).unwrap();
    let Input::List(items) = &parsed else { panic!("expected List variant") };
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[0], UserInputOrJson::Typed(UserInput::Text(_))));
    assert!(matches!(&items[1], UserInputOrJson::Typed(UserInput::LocalImage(_))));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn input_list_accepts_raw_json_dicts() {
    // Python's `List[Union[UserInput, Dict[str, Any]]]` lets callers pass raw
    // dicts that didn't go through TextInput/LocalImageInput construction.
    let wire = json!([{"type": "future_input_kind", "blob": [1, 2, 3]}]);
    let parsed: Input = serde_json::from_value(wire.clone()).unwrap();
    let Input::List(items) = &parsed else { panic!("expected List variant") };
    assert!(matches!(&items[0], UserInputOrJson::Raw(_)));
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

// ---------------------------------------------------------------------------
// CodexInstallResult — `install.py:19-23`
// ---------------------------------------------------------------------------

#[test]
fn codex_install_result_constructs() {
    let r = CodexInstallResult {
        codex_path: "/usr/local/bin/codex".into(),
        installed: true,
    };
    assert_eq!(r.codex_path, "/usr/local/bin/codex");
    assert!(r.installed);
}

// ---------------------------------------------------------------------------
// OutputSchemaFile — `output_schema_file.py:13-39`
// ---------------------------------------------------------------------------

#[test]
fn output_schema_file_constructs_with_path() {
    let f = OutputSchemaFile {
        schema_path: Some("/tmp/codex-output-schema-abc/schema.json".into()),
        dir: Some("/tmp/codex-output-schema-abc".into()),
    };
    assert!(f.schema_path.is_some());
    assert!(f.dir.is_some());
}

#[test]
fn output_schema_file_constructs_when_no_schema() {
    // Mirrors `OutputSchemaFile(schema_path=None, _dir=None)` from
    // `create_output_schema_file(schema=None)` in the Python SDK.
    let f = OutputSchemaFile { schema_path: None, dir: None };
    assert!(f.schema_path.is_none());
    assert!(f.dir.is_none());
}

// ---------------------------------------------------------------------------
// Abort family — pure data definitions, never serialized.
// ---------------------------------------------------------------------------

#[test]
fn abort_signal_default_is_unset() {
    let s = AbortSignal::default();
    assert!(!s.aborted);
    assert!(s.reason.is_none());
}

#[test]
fn abort_signal_carries_arbitrary_reason() {
    let s = AbortSignal {
        aborted: true,
        reason: Some(json!({"code": 408, "message": "user cancelled"})),
    };
    assert!(s.aborted);
    assert_eq!(s.reason.as_ref().unwrap()["code"], 408);
}

#[test]
fn abort_controller_owns_signal() {
    let c = AbortController::default();
    assert!(!c.signal.aborted);
}

#[test]
fn abort_reason_constructs() {
    let r = AbortReason { message: "user requested cancellation".into() };
    assert_eq!(r.message, "user requested cancellation");
}

#[test]
fn abort_error_implements_std_error() {
    let e = AbortError { message: "Operation aborted".into() };
    let as_err: &dyn std::error::Error = &e;
    assert_eq!(as_err.to_string(), "Operation aborted");
}

// ---------------------------------------------------------------------------
// TurnOptions.signal — `#[serde(skip)]`, never crosses the wire.
// ---------------------------------------------------------------------------

#[test]
fn turn_options_signal_does_not_serialize() {
    let opts = TurnOptions {
        output_schema: Some(json!({"type": "object"})),
        signal: Some(AbortSignal::default()),
    };
    let wire = serde_json::to_value(&opts).unwrap();
    assert!(wire.get("signal").is_none(), "signal must be skipped on serialize");
    assert_eq!(wire["output_schema"]["type"], "object");
}

#[test]
fn turn_options_deserialize_ignores_signal_and_keeps_deny_unknown_fields() {
    // signal is #[serde(skip)] — the wire must not contain it. Confirm a
    // payload with NO signal still parses cleanly (regression check that
    // skip didn't break round-trip), and a payload with an unknown OTHER
    // field still gets rejected by deny_unknown_fields.
    let ok: TurnOptions = serde_json::from_value(json!({"output_schema": {}})).unwrap();
    assert!(ok.signal.is_none());

    let err: Result<TurnOptions, _> =
        serde_json::from_value(json!({"output_schema": {}, "rogue": 1}));
    assert!(err.is_err(), "deny_unknown_fields still rejects unrecognised keys");
}

// ---------------------------------------------------------------------------
// Error enum — full Python parity check.
// ---------------------------------------------------------------------------

#[test]
fn error_variants_cover_all_codex_sdk_error_subclasses() {
    // Exercise each variant's Display so nothing rots silently.
    let parse: Error = serde_json::from_str::<serde_json::Value>("{").unwrap_err().into();
    assert!(parse.to_string().contains("failed to parse"));
    assert!(Error::Exec("nope".into()).to_string().contains("codex exec failed"));
    assert!(Error::ThreadRun("boom".into()).to_string().contains("thread run error"));
    assert!(Error::Install("nope".into()).to_string().contains("codex install failed"));
    assert!(Error::Auth("nope".into()).to_string().contains("codex auth failed"));
}

// ---------------------------------------------------------------------------
// Standalone leaf round-trips — confirm each discriminator-bearing struct
// works on its own (not just nested inside its parent enum). This is the
// fix for the bug where TextInput/LocalImageInput dropped the `type` field.
// ---------------------------------------------------------------------------

#[test]
fn text_input_round_trips_standalone() {
    let wire = json!({"type": "text", "text": "hi"});
    let parsed: TextInput = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed.r#type, TextInputType::Text);
    assert_eq!(parsed.text, "hi");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn text_input_rejects_wrong_type_literal() {
    let wire = json!({"type": "local_image", "text": "hi"});
    let result: Result<TextInput, _> = serde_json::from_value(wire);
    assert!(result.is_err(), "TextInput must reject the wrong type literal");
}

#[test]
fn local_image_input_round_trips_standalone() {
    let wire = json!({"type": "local_image", "path": "/tmp/x.png"});
    let parsed: LocalImageInput = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed.r#type, LocalImageInputType::LocalImage);
    assert_eq!(parsed.path, "/tmp/x.png");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn local_image_input_rejects_wrong_type_literal() {
    let wire = json!({"type": "text", "path": "/tmp/x.png"});
    let result: Result<LocalImageInput, _> = serde_json::from_value(wire);
    assert!(result.is_err(), "LocalImageInput must reject the wrong type literal");
}

#[test]
fn agent_message_item_round_trips_standalone() {
    let wire = json!({"id": "i", "type": "agent_message", "text": "hi"});
    let parsed: AgentMessageItem = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed.r#type, AgentMessageItemType::AgentMessage);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn command_execution_item_round_trips_standalone() {
    let wire = json!({
        "id": "c",
        "type": "command_execution",
        "command": "ls",
        "aggregated_output": "",
        "status": "in_progress",
    });
    let parsed: CommandExecutionItem = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed.r#type, CommandExecutionItemType::CommandExecution);
    assert_eq!(parsed.exit_code, None);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn mcp_tool_call_item_rejects_wrong_type_literal() {
    let wire = json!({
        "id": "t",
        "type": "agent_message",  // wrong literal
        "server": "fs",
        "tool": "read_file",
        "arguments": {},
        "status": "in_progress",
    });
    let result: Result<McpToolCallItem, _> = serde_json::from_value(wire);
    assert!(result.is_err());
}

#[test]
fn thread_started_event_round_trips_standalone() {
    let wire = json!({"type": "thread.started", "thread_id": "thr"});
    let parsed: ThreadStartedEvent = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed.r#type, ThreadStartedEventType::ThreadStarted);
    assert_eq!(parsed.thread_id, "thr");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn turn_completed_event_round_trips_standalone() {
    let wire = json!({
        "type": "turn.completed",
        "usage": {"input_tokens": 1, "cached_input_tokens": 0, "output_tokens": 2},
    });
    let parsed: TurnCompletedEvent = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(parsed.r#type, TurnCompletedEventType::TurnCompleted);
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[test]
fn turn_completed_event_rejects_wrong_type_literal() {
    let wire = json!({
        "type": "turn.failed",
        "usage": {"input_tokens": 1, "cached_input_tokens": 0, "output_tokens": 2},
    });
    let result: Result<TurnCompletedEvent, _> = serde_json::from_value(wire);
    assert!(result.is_err());
}
