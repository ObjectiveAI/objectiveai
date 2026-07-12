//! E2E: laboratory `create` runtime effects — custom cwd, env vars, and
//! host filesystem mounts — exercised through the CLI.
//!
//! Each test creates a lab, attaches it to a GROUPED tag, spawns a mock
//! agent whose deterministic `calls` script runs the lab's `Bash` tool,
//! and asserts on the tool output (and, for mounts, the host fs).

mod cli_test_util;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
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
use objectiveai_sdk::cli::command::laboratories::create::{
    EnvVar, Kind, Mount, Path as CreatePath, Request as CreateReq, Response as CreateResp,
};
use serde_json::json;

const BASE_IMAGE: &str = "docker.io/library/bash:latest";

type Exec = cli_test_util::HangPreventingBinaryCommandExecutor;

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// Create a laboratory with explicit mounts/env/cwd. Creation runs on
/// this machine's laboratory HOST (auto-spawned by the daemon when
/// absent) and announces the lab to the registry — no separate
/// connect step exists; the container starts lazily on its first
/// routed op.
async fn create_lab(
    executor: &Exec,
    id: &str,
    mounts: Vec<Mount>,
    env: Vec<EnvVar>,
    cwd: &str,
) {
    let created: CreateResp = cli_test_util::execute_one(
        executor,
        CreateReq {
            path_type: CreatePath::LaboratoriesCreate,
            kind: Kind::Client,
            id: id.to_string(),
            image: BASE_IMAGE.to_string(),
            mounts,
            env,
            cwd: cwd.to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(created.id, id);
}

/// Apply `tag` carrying `agent_json` (a GROUPED mock agent), attach `labs`
/// to it, spawn via the tag, wait, and return the response id.
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

/// Build a one-turn agent that calls the lab's `Bash` tool with `command`.
fn bash_agent(lab_id: &str, command: &str) -> serde_json::Value {
    let bash_tool = {
        use objectiveai_sdk::laboratories::{ClientLaboratory, ClientLaboratoryType};
        let server_name = ClientLaboratory {
            r#type: ClientLaboratoryType::Client,
            id: lab_id.to_string(),
            machine: Some(objectiveai_sdk::machine::machine_id(
                &cli_test_util::objectiveai_dir(),
            )),
            machine_state: Some(cli_test_util::test_state_name()),
        }
        .server_name()
        .expect("machine + state present");
        format!("{server_name}_Bash")
    };
    let args = serde_json::to_string(&json!({ "command": command })).unwrap();
    json!({
        "upstream": "mock",
        "output_mode": "instruction",
        "instruction": "done",
        "calls": [
            { "tool_calls": [{ "name": bash_tool, "arguments": args }], "content": "" }
        ]
    })
}

/// `--cwd` baked at create time is where the lab's first command runs.
#[tokio::test(flavor = "multi_thread")]
async fn custom_cwd_takes_effect() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let n = nanos();
    let lab = format!("cwd-{n}");
    let tag = format!("cwd-tag-{n}");
    create_lab(&executor, &lab, Vec::new(), Vec::new(), "/tmp").await;

    let rid = spawn_lab_session(&executor, &tag, bash_agent(&lab, "pwd"), &[&lab]).await;
    let results = tool_result_texts(&executor, &rid).await.join("\n");
    assert!(
        results.contains("/tmp"),
        "expected the lab's bash to start in the custom cwd /tmp; got: {results}"
    );
}

/// `--env K=V` baked at create time is visible to the lab's bash.
#[tokio::test(flavor = "multi_thread")]
async fn env_vars_available_in_bash() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let n = nanos();
    let lab = format!("env-{n}");
    let tag = format!("env-tag-{n}");
    create_lab(
        &executor,
        &lab,
        Vec::new(),
        vec![EnvVar {
            key: "OAI_LAB_MARKER".to_string(),
            value: "marker-value-123".to_string(),
        }],
        "/",
    )
    .await;

    let rid = spawn_lab_session(
        &executor,
        &tag,
        bash_agent(&lab, "printf %s \"$OAI_LAB_MARKER\""),
        &[&lab],
    )
    .await;
    let results = tool_result_texts(&executor, &rid).await.join("\n");
    assert!(
        results.contains("marker-value-123"),
        "expected the baked env var to be readable in bash; got: {results}"
    );
}

/// A host directory mounted into the laboratory round-trips both ways:
/// host-seeded content is readable inside, and the laboratory's writes
/// persist back to the host path.
#[tokio::test(flavor = "multi_thread")]
async fn host_mount_round_trips() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let n = nanos();
    // Random-named host temp dir, seeded before the mount.
    let host_dir = std::env::temp_dir().join(format!("oail-mount-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&host_dir).expect("create host mount dir");
    std::fs::write(host_dir.join("seed.txt"), b"seed-content-xyz").expect("seed file");

    let lab = format!("mnt-{n}");
    let tag = format!("mnt-tag-{n}");
    create_lab(
        &executor,
        &lab,
        vec![Mount {
            host: host_dir.to_string_lossy().into_owned(),
            container: "/mnt/data".to_string(),
        }],
        Vec::new(),
        "/",
    )
    .await;

    // Read the host-seeded file in the laboratory, then write back to the mount.
    let rid = spawn_lab_session(
        &executor,
        &tag,
        bash_agent(
            &lab,
            "cat /mnt/data/seed.txt && printf written-from-lab > /mnt/data/out.txt",
        ),
        &[&lab],
    )
    .await;
    let results = tool_result_texts(&executor, &rid).await.join("\n");

    // host -> laboratory visibility.
    assert!(
        results.contains("seed-content-xyz"),
        "expected the host-seeded file to be readable inside the laboratory; got: {results}"
    );
    // laboratory -> host persistence.
    let out = std::fs::read_to_string(host_dir.join("out.txt")).unwrap_or_default();
    assert_eq!(
        out, "written-from-lab",
        "expected the laboratory's write to persist to the host mount"
    );

    let _ = std::fs::remove_dir_all(&host_dir);
}
