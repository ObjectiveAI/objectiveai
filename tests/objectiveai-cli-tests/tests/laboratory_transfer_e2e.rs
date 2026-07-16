//! E2E: the proxy-native `laboratory_transfer` tool, end to end through
//! the CLI with two laboratories.
//!
//! Everything goes through the CLI:
//!   1. `laboratories create` two labs (a, b).
//!   2. `agents tags apply` a GROUPED tag carrying a mock agent whose
//!      deterministic `calls` script drives the aggregated tools.
//!   3. `laboratories attach` both labs to the tag so the spawn
//!      resolves them into the session.
//!   4. `agents spawn` the tag. The scripted agent:
//!        - writes `/work/x = "hi"` in lab a via its `Bash` tool,
//!        - calls `laboratory_transfer` a -> b,
//!        - reads `/work/x` back in lab b via its `Bash` tool.
//!   5. Assert the tool-response text shows the transfer succeeded and
//!      lab b now has the file.
//!
//! A laboratory's server is named `oail-<base62(fnv1a32(composite))>`
//! (a fixed hash token — the raw id has arbitrary length/charset and
//! cannot ride tool names); the proxy routing prefix is that verbatim,
//! so the aggregated bash tool is `<server-name>_Bash`
//! ([`ClientLaboratory::server_name`]). `laboratory_transfer` takes
//! laboratory ids for source/destination.

mod cli_test_util;

use std::path::PathBuf;
use std::process::Command;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
};
use objectiveai_sdk::cli::command::laboratories::detach::{
    Path as DetachPath, Request as DetachReq, Response as DetachResp,
};
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::spawn::{
    Path as SpawnPath, Request as SpawnReq, RequestDangerousAdvanced,
    ResponseItem as SpawnItem,
};
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyReq, Response as ApplyResp, Target as ApplyTarget,
};
use objectiveai_sdk::cli::command::SetScope;
use objectiveai_sdk::cli::command::laboratories::config::addresses::add::{
    Path as AddrAddPath, Request as AddrAddReq, Response as AddrAddResp,
};
use objectiveai_sdk::cli::command::laboratories::create::{
    Kind, Path as CreatePath, Request as CreateReq, Response as CreateResp,
};
use serde_json::json;

/// A base image with `/bin/bash` + coreutils for the laboratory's `Bash`
/// tool. The official `bash` image is Alpine-based (musl).
/// The split base image every lab in this file uses —
/// `docker.io/library/bash:latest`, as parts (a joined reference string
/// is unrepresentable in the API).
fn base_image() -> objectiveai_sdk::laboratories::LaboratoryImage {
    objectiveai_sdk::laboratories::LaboratoryImage::Registry(
        objectiveai_sdk::laboratories::RegistryLaboratoryImage {
        registry: "docker.io".to_string(),
        name: "library/bash".to_string(),
            pin: objectiveai_sdk::laboratories::LaboratoryImagePin::Tag(
                "latest".to_string(),
            ),
        },
    )
}

/// RAII kill of the plugin process (PID read from `OAI_TEST_MCP_PID_FILE`)
/// on test drop — mirrors `plugin_mcp_self_call_e2e`.
struct PluginGuard {
    pid_file: PathBuf,
}

impl Drop for PluginGuard {
    fn drop(&mut self) {
        if let Ok(s) = std::fs::read_to_string(&self.pid_file) {
            if let Ok(pid) = s.trim().parse::<u32>() {
                #[cfg(windows)]
                let _ = Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .status();
                #[cfg(unix)]
                let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            }
        }
    }
}

type Exec = cli_test_util::HangPreventingBinaryCommandExecutor;

/// Create a laboratory. Creation runs on this machine's laboratory
/// HOST (auto-spawned by the daemon when absent) and announces the lab
/// to the registry — no separate connect step exists; the container
/// starts lazily on its first routed op.
async fn create_lab(executor: &Exec, id: &str) {
    let created: CreateResp = cli_test_util::execute_one(
        executor,
        CreateReq {
            path_type: CreatePath::LaboratoriesCreate,
            kind: Kind::Client,
            id: id.to_string(),
            image: base_image(),
            mounts: Vec::new(),
            env: Vec::new(),
            // Default cwd `/` — it always exists. The lab's first bash
            // spawns in `default_cwd`, so a non-existent dir (e.g. /work,
            // absent from the base image) would fail before any command;
            // the scripts `mkdir -p` their own work dirs.
            cwd: "/".to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(created.id, id);
}

/// The COMPOSITE laboratory id the assistant must quote —
/// `{machine}/{base62(state)}/{base62(id)}` for a lab on THIS
/// machine + this test's state (where every default-pair create
/// lands).
fn lab_marker(lab: &str) -> objectiveai_sdk::laboratories::ClientLaboratory {
    use objectiveai_sdk::laboratories::{ClientLaboratory, ClientLaboratoryType};
    ClientLaboratory {
        r#type: ClientLaboratoryType::Client,
        id: lab.to_string(),
        machine: Some(objectiveai_sdk::machine::machine_id(
            &cli_test_util::objectiveai_dir(),
        )),
        machine_state: Some(cli_test_util::test_state_name()),
    }
}

fn composite_id(lab: &str) -> String {
    lab_marker(lab).composite_id().expect("machine + state present")
}

/// The lab's MCP server name — `oail-<hash of the composite>`; its
/// `Bash` tool is `<server_name>_Bash`.
fn server_name(lab: &str) -> String {
    lab_marker(lab).server_name().expect("machine + state present")
}

async fn attach_lab(executor: &Exec, tag: &str, lab: &str) {
    let _: AttachResp = cli_test_util::execute_one(
        executor,
        AttachReq {
            path_type: AttachPath::LaboratoriesAttach,
            selector: AgentSelector::Tag {
                agent_tag: tag.to_string(),
            },
            laboratory_id: lab.to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
}

/// Pull every `tool_response_content_text.text` row for `response_id`.
async fn tool_result_texts(executor: &Exec, response_id: &str) -> Vec<String> {
    let sql = format!(
        "SELECT text FROM objectiveai.tool_response_content_text \
         WHERE response_id = '{}' ORDER BY \"index\", part_index",
        response_id.replace('\'', "''"),
    );
    cli_test_util::db_query(executor, &sql)
        .await
        .into_iter()
        .filter_map(|mut row| row.pop())
        .filter_map(|v| match v {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn transfer_between_two_laboratories() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let lab_a = format!("lt-a-{nanos}");
    let lab_b = format!("lt-b-{nanos}");
    let tag = format!("lt-tag-{nanos}");

    create_lab(&executor, &lab_a).await;
    create_lab(&executor, &lab_b).await;

    // Deterministic agent script: write in a -> transfer a->b -> read in b.
    let bash_a = format!("{}_Bash", server_name(&lab_a));
    let bash_b = format!("{}_Bash", server_name(&lab_b));
    let write_args =
        serde_json::to_string(&json!({ "command": "mkdir -p /work && printf hi > /work/x" }))
            .unwrap();
    let transfer_args = serde_json::to_string(&json!({
        "source": composite_id(&lab_a),
        "source_path": "/work/x",
        "destination": composite_id(&lab_b),
        "destination_path": "/work",
    }))
    .unwrap();
    let read_args = serde_json::to_string(&json!({ "command": "cat /work/x" })).unwrap();

    let agent_spec = serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
        json!({
            "upstream": "mock",
            "output_mode": "instruction",
            "instruction": "done",
            "calls": [
                { "tool_calls": [{ "name": bash_a, "arguments": write_args }], "content": "" },
                { "tool_calls": [{ "name": "laboratory_transfer", "arguments": transfer_args }], "content": "" },
                { "tool_calls": [{ "name": bash_b, "arguments": read_args }], "content": "" }
            ]
        }),
    )
    .expect("mock agent spec deserializes");

    // GROUPED tag carrying that agent; attach both labs to it.
    let _: ApplyResp = cli_test_util::execute_one(
        &executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.clone(),
            target: ApplyTarget::Agent {
                agent_spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
    attach_lab(&executor, &tag, &lab_a).await;
    attach_lab(&executor, &tag, &lab_b).await;

    // Spawn the tag; the labs resolve into the session.
    let items: Vec<SpawnItem> = cli_test_util::collect_stream(
        &executor,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple("transfer a file".to_string()),
            agent: AgentSelector::Tag {
                agent_tag: tag.clone(),
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(1),
            }),
            base: Default::default(),
        },
    )
    .await;

    let aih = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("spawn emits an agent_instance_hierarchy");
    let response_id = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.id.is_empty() => Some(c.id.clone()),
            _ => None,
        })
        .expect("spawn emits a response id");

    cli_test_util::wait_for_agent(&executor, &aih).await;

    let results = tool_result_texts(&executor, &response_id).await.join("\n");
    assert!(
        results.contains("transferred"),
        "expected a laboratory_transfer success line; got: {results}"
    );
    assert!(
        results.contains("hi"),
        "expected lab b to read back the transferred file content 'hi'; got: {results}"
    );
}

/// `agents mcp servers list` shows both labs (with their ids), and
/// `agents mcp tools list --name <lab>` scopes to that one lab — driven
/// in-session by the `lab-driver` plugin surface (re-invoking the CLI for
/// the live response id), so it's all through the CLI.
#[tokio::test(flavor = "multi_thread")]
async fn servers_list_and_name_filter() {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };
    let executor = cli_test_util::executor()
        .await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let lab_a = format!("ls-a-{nanos}");
    let lab_b = format!("ls-b-{nanos}");
    let tag = format!("ls-tag-{nanos}");

    create_lab(&executor, &lab_a).await;
    create_lab(&executor, &lab_b).await;

    let tools_named_args =
        serde_json::to_string(&json!({ "name": server_name(&lab_a) })).unwrap();
    let agent_spec = serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
        json!({
            "upstream": "mock",
            "output_mode": "instruction",
            "instruction": "done",
            "client_objectiveai_mcp": {
                "plugins": [{
                    "owner": "testorg",
                    "name": "test-mcp-plugin-self-call",
                    "version": "1.0.0",
                    "executable": false,
                    "mcp_servers": [{ "name": "lab-driver" }]
                }]
            },
            "calls": [
                { "tool_calls": [{ "name": "test-mcp-plugin-self-call_do_servers_list", "arguments": "{}" }], "content": "" },
                { "tool_calls": [{ "name": "test-mcp-plugin-self-call_do_tools_named", "arguments": tools_named_args }], "content": "" }
            ]
        }),
    )
    .expect("mock agent spec deserializes");

    let _: ApplyResp = cli_test_util::execute_one(
        &executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.clone(),
            target: ApplyTarget::Agent {
                agent_spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
    attach_lab(&executor, &tag, &lab_a).await;
    attach_lab(&executor, &tag, &lab_b).await;

    let items: Vec<SpawnItem> = cli_test_util::collect_stream(
        &executor,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple("inspect servers".to_string()),
            agent: AgentSelector::Tag {
                agent_tag: tag.clone(),
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(1),
            }),
            base: Default::default(),
        },
    )
    .await;

    let aih = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("spawn emits an agent_instance_hierarchy");
    let response_id = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.id.is_empty() => Some(c.id.clone()),
            _ => None,
        })
        .expect("spawn emits a response id");

    cli_test_util::wait_for_agent(&executor, &aih).await;

    let results = tool_result_texts(&executor, &response_id).await.join("\n");
    // servers list shows both labs by their ids.
    assert!(
        results.contains(&lab_a) && results.contains(&lab_b),
        "servers list should report both lab ids; got: {results}"
    );
    // tools --name scopes to lab a's Bash tool; lab b's prefix is absent
    // from the filtered listing.
    assert!(
        results.contains(&server_name(&lab_a)) && results.contains("Bash"),
        "tools --name should include lab a's Bash tool; got: {results}"
    );
    assert!(
        !results.contains(&format!("{}_Bash", server_name(&lab_b))),
        "tools --name <lab a> must not include lab b's tools; got: {results}"
    );
}

/// Apply `tag` carrying `agent_json` (a GROUPED mock agent), attach `labs`
/// to it, spawn via the tag, wait for completion, and return the response
/// id. Encapsulates the create→tag→attach→spawn→wait orchestration.
async fn spawn_lab_session(
    executor: &Exec,
    tag: &str,
    agent_json: serde_json::Value,
    labs: &[&str],
) -> String {
    let agent_spec =
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(agent_json)
            .expect("mock agent spec deserializes");
    let _: ApplyResp = cli_test_util::execute_one(
        executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.to_string(),
            target: ApplyTarget::Agent {
                agent_spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
    for lab in labs {
        attach_lab(executor, tag, lab).await;
    }
    let items: Vec<SpawnItem> = cli_test_util::collect_stream(
        executor,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple("go".to_string()),
            agent: AgentSelector::Tag {
                agent_tag: tag.to_string(),
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(1),
            }),
            base: Default::default(),
        },
    )
    .await;
    let aih = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("spawn emits an agent_instance_hierarchy");
    let response_id = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.id.is_empty() => Some(c.id.clone()),
            _ => None,
        })
        .expect("spawn emits a response id");
    cli_test_util::wait_for_agent(executor, &aih).await;
    response_id
}

/// `laboratory_transfer` of a directory tree (a -> b), nested files
/// restored cp-style under `<dest>/<basename>`.
#[tokio::test(flavor = "multi_thread")]
async fn transfer_directory_between_laboratories() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let lab_a = format!("td-a-{nanos}");
    let lab_b = format!("td-b-{nanos}");
    let tag = format!("td-tag-{nanos}");
    create_lab(&executor, &lab_a).await;
    create_lab(&executor, &lab_b).await;

    let bash_a = format!("{}_Bash", server_name(&lab_a));
    let bash_b = format!("{}_Bash", server_name(&lab_b));
    let write_args = serde_json::to_string(&json!({
        "command": "mkdir -p /work/d/sub && printf a > /work/d/f1 && printf b > /work/d/sub/f2"
    }))
    .unwrap();
    let transfer_args = serde_json::to_string(&json!({
        "source": composite_id(&lab_a), "source_path": "/work/d",
        "destination": composite_id(&lab_b), "destination_path": "/work",
    }))
    .unwrap();
    // Concatenate both files with a separator so the result is distinctive.
    let read_args =
        serde_json::to_string(&json!({ "command": "cat /work/d/f1; printf '|'; cat /work/d/sub/f2" }))
            .unwrap();

    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": "done",
        "calls": [
            { "tool_calls": [{ "name": bash_a, "arguments": write_args }], "content": "" },
            { "tool_calls": [{ "name": "laboratory_transfer", "arguments": transfer_args }], "content": "" },
            { "tool_calls": [{ "name": bash_b, "arguments": read_args }], "content": "" }
        ]
    });
    let rid = spawn_lab_session(&executor, &tag, agent_json, &[&lab_a, &lab_b]).await;

    let results = tool_result_texts(&executor, &rid).await.join("\n");
    assert!(
        results.contains("transferred"),
        "expected a laboratory_transfer success line; got: {results}"
    );
    assert!(
        results.contains("a|b"),
        "expected lab b to read back the transferred directory's nested files; got: {results}"
    );
}

/// `laboratory_transfer` with an unknown source laboratory id surfaces as
/// an `isError` tool result (the resolve-by-id failure path).
#[tokio::test(flavor = "multi_thread")]
async fn transfer_unknown_laboratory_is_error() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let lab_a = format!("te-a-{nanos}");
    let lab_b = format!("te-b-{nanos}");
    let tag = format!("te-tag-{nanos}");
    create_lab(&executor, &lab_a).await;
    create_lab(&executor, &lab_b).await;

    // Turn 1: a WELL-FORMED composite that matches nothing (this
    // machine + state, nonexistent raw id) — the resolve failure.
    // Turn 2: a MALFORMED bare id — the composite-format rejection.
    let unknown_args = serde_json::to_string(&json!({
        "source": composite_id("does-not-exist"),
        "source_path": "/work/x",
        "destination": composite_id(&lab_b),
        "destination_path": "/work",
    }))
    .unwrap();
    let malformed_args = serde_json::to_string(&json!({
        "source": "does-not-exist",
        "source_path": "/work/x",
        "destination": composite_id(&lab_b),
        "destination_path": "/work",
    }))
    .unwrap();
    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": "done",
        "calls": [
            { "tool_calls": [{ "name": "laboratory_transfer", "arguments": unknown_args }], "content": "" },
            { "tool_calls": [{ "name": "laboratory_transfer", "arguments": malformed_args }], "content": "" }
        ]
    });
    let rid = spawn_lab_session(&executor, &tag, agent_json, &[&lab_a, &lab_b]).await;

    let results = tool_result_texts(&executor, &rid).await.join("\n");
    assert!(
        results.contains("no laboratory"),
        "unknown source laboratory should error; got: {results}"
    );
    assert!(
        results.contains("not a composite laboratory id"),
        "a bare id must be rejected with the composite-format error; got: {results}"
    );
}

/// `laboratory_transfer` is NOT injected when a session has fewer than two
/// laboratories. Inspect the aggregated tool list (via the plugin's
/// `list-tools` surface) in a one-lab session.
#[tokio::test(flavor = "multi_thread")]
async fn laboratory_transfer_absent_with_one_lab() {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };
    let executor = cli_test_util::executor()
        .await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let lab_a = format!("v1-a-{nanos}");
    let tag = format!("v1-tag-{nanos}");
    create_lab(&executor, &lab_a).await;

    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": "done",
        "client_objectiveai_mcp": {
            "plugins": [{
                "owner": "testorg",
                "name": "test-mcp-plugin-self-call",
                "version": "1.0.0",
                "executable": false,
                "mcp_servers": [{ "name": "list-tools" }]
            }]
        },
        "calls": [
            { "tool_calls": [{ "name": "test-mcp-plugin-self-call_do_list_tools", "arguments": "{}" }], "content": "" }
        ]
    });
    let rid = spawn_lab_session(&executor, &tag, agent_json, &[&lab_a]).await;

    let results = tool_result_texts(&executor, &rid).await.join("\n");
    assert!(
        !results.contains("laboratory_transfer"),
        "laboratory_transfer must be absent with fewer than two labs; got: {results}"
    );
    assert!(
        results.contains("Bash"),
        "expected the single lab's Bash tool in the listing; got: {results}"
    );
}

/// (Re)apply `tag` as a GROUPED tag carrying `agent_json`. Re-applying is
/// an upsert that resets a previously-bound tag back to GROUPED, so the
/// next spawn is a fresh session (its lab attachments, keyed by tag name,
/// survive the reset).
async fn apply_grouped_tag(executor: &Exec, tag: &str, agent_json: serde_json::Value) {
    let agent_spec =
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(agent_json)
            .expect("mock agent spec deserializes");
    let _: ApplyResp = cli_test_util::execute_one(
        executor,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.to_string(),
            target: ApplyTarget::Agent {
                agent_spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
}

async fn detach_lab(executor: &Exec, tag: &str, lab: &str) {
    let _: DetachResp = cli_test_util::execute_one(
        executor,
        DetachReq {
            path_type: DetachPath::LaboratoriesDetach,
            selector: AgentSelector::Tag {
                agent_tag: tag.to_string(),
            },
            laboratory_id: lab.to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
}

/// Spawn `tag`, wait for the agent to finish, and return its response id.
async fn spawn_tag(executor: &Exec, tag: &str) -> String {
    let items: Vec<SpawnItem> = cli_test_util::collect_stream(
        executor,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple("inspect servers".to_string()),
            agent: AgentSelector::Tag {
                agent_tag: tag.to_string(),
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(1),
            }),
            base: Default::default(),
        },
    )
    .await;
    let aih = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("spawn emits an agent_instance_hierarchy");
    let rid = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.id.is_empty() => Some(c.id.clone()),
            _ => None,
        })
        .expect("spawn emits a response id");
    cli_test_util::wait_for_agent(executor, &aih).await;
    rid
}

/// `agents mcp servers list` reflects attach/detach across successive
/// sessions: one lab present, then a second attached → both present, then
/// one detached → only the remaining one. Each session is a fresh spawn of
/// the same tag (re-applied to GROUPED between spawns) running a single
/// `do_servers_list` against the tag's current lab attachments.
#[tokio::test(flavor = "multi_thread")]
async fn servers_list_reflects_attach_and_detach() {
    let base = cli_test_util::test_base_dir();
    let pid_file = base.join("plugin-pid");
    let _guard = PluginGuard {
        pid_file: pid_file.clone(),
    };
    let executor = cli_test_util::executor()
        .await
        .env("OAI_TEST_MCP_PID_FILE", pid_file.to_string_lossy().into_owned());

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let lab_a = format!("dyn-a-{nanos}");
    let lab_b = format!("dyn-b-{nanos}");
    let tag = format!("dyn-tag-{nanos}");
    create_lab(&executor, &lab_a).await;
    create_lab(&executor, &lab_b).await;

    // A lab-driver plugin agent that does exactly one `do_servers_list` per
    // (fresh) session.
    let agent_json = || {
        json!({
            "upstream": "mock",
            "output_mode": "instruction",
            "instruction": "done",
            "client_objectiveai_mcp": {
                "plugins": [{
                    "owner": "testorg",
                    "name": "test-mcp-plugin-self-call",
                    "version": "1.0.0",
                    "executable": false,
                    "mcp_servers": [{ "name": "lab-driver" }]
                }]
            },
            "calls": [
                { "tool_calls": [{ "name": "test-mcp-plugin-self-call_do_servers_list", "arguments": "{}" }], "content": "" }
            ]
        })
    };

    // Session 1: only lab A attached.
    apply_grouped_tag(&executor, &tag, agent_json()).await;
    attach_lab(&executor, &tag, &lab_a).await;
    let s1 = tool_result_texts(&executor, &spawn_tag(&executor, &tag).await)
        .await
        .join("\n");
    assert!(
        s1.contains(&lab_a),
        "session 1 servers list should include lab A; got: {s1}"
    );
    assert!(
        !s1.contains(&lab_b),
        "session 1 should not yet include lab B; got: {s1}"
    );

    // Session 2: attach lab B → both present.
    apply_grouped_tag(&executor, &tag, agent_json()).await;
    attach_lab(&executor, &tag, &lab_b).await;
    let s2 = tool_result_texts(&executor, &spawn_tag(&executor, &tag).await)
        .await
        .join("\n");
    assert!(
        s2.contains(&lab_a) && s2.contains(&lab_b),
        "session 2 servers list should include both labs; got: {s2}"
    );

    // Session 3: detach lab A → only lab B remains.
    apply_grouped_tag(&executor, &tag, agent_json()).await;
    detach_lab(&executor, &tag, &lab_a).await;
    let s3 = tool_result_texts(&executor, &spawn_tag(&executor, &tag).await)
        .await
        .join("\n");
    assert!(
        s3.contains(&lab_b),
        "session 3 servers list should still include lab B; got: {s3}"
    );
    assert!(
        !s3.contains(&lab_a),
        "session 3 should no longer include lab A; got: {s3}"
    );
}

/// Cross-STATE transfer: two laboratories on the SAME machine but in
/// two DIFFERENT states — two daemons, two hosts. The (machine, state)
/// pairs differ, so the proxy sends the plain `transfer` request and
/// the CLI daemon drives the export→import splice between the two
/// hosts (the only e2e coverage that path gets — same-state transfers
/// collapse to `local_transfer` inside one host).
#[tokio::test(flavor = "multi_thread")]
async fn transfer_across_states() {
    let _base = cli_test_util::test_base_dir();
    let state_a = cli_test_util::test_state_name();
    let state_b = format!("{state_a}-b");
    let exec_a = cli_test_util::executor().await;
    let exec_b = cli_test_util::executor_for_state(&state_b).await;

    // The agent session runs in daemon A, so state B's HOST must dial
    // daemon A too (that is how A's registry learns the (machine,
    // state B) host). Configure the extra address BEFORE anything
    // spawns B's host, then create B's lab (create auto-spawns the
    // host with the config in effect).
    let addr_a = cli_test_util::daemon_address(&exec_a, &state_a).await;
    let addr_b = cli_test_util::daemon_address(&exec_b, &state_b).await;
    assert_ne!(addr_a, addr_b, "two daemons must bind distinct ports");
    let _: AddrAddResp = cli_test_util::execute_one(
        &exec_b,
        AddrAddReq {
            path_type: AddrAddPath::LaboratoriesConfigAddressesAdd,
            scope: SetScope::State,
            key: addr_a.clone(),
            value: String::new(),
            base: Default::default(),
        },
    )
    .await;

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let lab_a = format!("xstate-a-{nanos}");
    let lab_b = format!("xstate-b-{nanos}");
    let tag = format!("xstate-tag-{nanos}");

    create_lab(&exec_a, &lab_a).await;
    create_lab(&exec_b, &lab_b).await;

    // Full identities: lab_a lives in state A, lab_b in state B —
    // their composites differ in the state segment, which is exactly
    // what routes the proxy onto the non-local path.
    let machine = objectiveai_sdk::machine::machine_id(&cli_test_util::objectiveai_dir());
    let marker = |lab: &str, state: &str| objectiveai_sdk::laboratories::ClientLaboratory {
        r#type: objectiveai_sdk::laboratories::ClientLaboratoryType::Client,
        id: lab.to_string(),
        machine: Some(machine.clone()),
        machine_state: Some(state.to_string()),
    };
    let marker_a = marker(&lab_a, &state_a);
    let marker_b = marker(&lab_b, &state_b);
    let composite_a = marker_a.composite_id().expect("machine + state present");
    let composite_b = marker_b.composite_id().expect("machine + state present");
    let bash_a = format!("{}_Bash", marker_a.server_name().expect("server name"));
    let bash_b = format!("{}_Bash", marker_b.server_name().expect("server name"));

    // Deterministic agent script: write in a → transfer a→b → read in b.
    let write_args = serde_json::to_string(
        &json!({ "command": "mkdir -p /work && printf xhi > /work/x" }),
    )
    .unwrap();
    let transfer_args = serde_json::to_string(&json!({
        "source": composite_a,
        "source_path": "/work/x",
        "destination": composite_b,
        "destination_path": "/work",
    }))
    .unwrap();
    let read_args = serde_json::to_string(&json!({ "command": "cat /work/x" })).unwrap();
    let agent_json = json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": "done",
        "calls": [
            { "tool_calls": [{ "name": bash_a, "arguments": write_args }], "content": "" },
            { "tool_calls": [{ "name": "laboratory_transfer", "arguments": transfer_args }], "content": "" },
            { "tool_calls": [{ "name": bash_b, "arguments": read_args }], "content": "" }
        ]
    });
    let agent_spec = serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(
        agent_json,
    )
    .expect("mock agent spec deserializes");

    // GROUPED tag; attach lab_a (defaults = state A) and lab_b with
    // its EXPLICIT (machine, state B) pair.
    let _: ApplyResp = cli_test_util::execute_one(
        &exec_a,
        ApplyReq {
            path_type: ApplyPath::AgentsTagsApply,
            name: tag.clone(),
            target: ApplyTarget::Agent {
                agent_spec,
                parent_agent_instance_hierarchy: None,
            },
            base: Default::default(),
        },
    )
    .await;
    attach_lab(&exec_a, &tag, &lab_a).await;
    let _: AttachResp = cli_test_util::execute_one(
        &exec_a,
        AttachReq {
            path_type: AttachPath::LaboratoriesAttach,
            selector: AgentSelector::Tag {
                agent_tag: tag.clone(),
            },
            laboratory_id: lab_b.clone(),
            machine: Some(machine.clone()),
            machine_state: Some(state_b.clone()),
            base: Default::default(),
        },
    )
    .await;

    // Spawn via daemon A; the session dials lab_a on A's host and
    // lab_b on B's host (reachable because B's host dials daemon A).
    let items: Vec<SpawnItem> = cli_test_util::collect_stream(
        &exec_a,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple("transfer across states".to_string()),
            agent: AgentSelector::Tag {
                agent_tag: tag.clone(),
            },
            dangerous_advanced: Some(RequestDangerousAdvanced {
                stream: Some(true),
                seed: Some(1),
            }),
            base: Default::default(),
        },
    )
    .await;

    let aih = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.agent_instance_hierarchy.is_empty() => {
                Some(c.agent_instance_hierarchy.clone())
            }
            _ => None,
        })
        .expect("spawn emits an agent_instance_hierarchy");
    let response_id = items
        .iter()
        .find_map(|i| match i {
            SpawnItem::Chunk(c) if !c.id.is_empty() => Some(c.id.clone()),
            _ => None,
        })
        .expect("spawn emits a response id");

    cli_test_util::wait_for_agent(&exec_a, &aih).await;

    let results = tool_result_texts(&exec_a, &response_id).await.join("\n");
    assert!(
        results.contains("transferred"),
        "expected a laboratory_transfer success line; got: {results}"
    );
    assert!(
        results.contains("xhi"),
        "expected lab b (state B) to read back the transferred content 'xhi'; got: {results}"
    );
}
