//! E2E: the daemon's `GET /laboratories/{id}/filetree` SSE endpoint —
//! the lab MCP's own `/filetree` contract (snapshot then deltas),
//! re-served from the daemon's materialized state (fed by the host's
//! pushed `laboratory_filetree` notifications; nothing polls).
//!
//! SCRIPT agents do the filesystem work: each test's agent is an
//! `upstream: "script"` python that branches on the incoming message
//! ("remove" ⇒ delete the marker, else create files) and drives the
//! laboratory's `Bash` tool via `objectiveai.execute(["agents","mcp",
//! "tools","call",…])` — one agent spec, two spawns, so the embedded
//! test's content-addressed derived laboratory id stays IDENTICAL
//! across both phases.
//!
//! Two laboratories × two observation paths:
//! - `created_attached_lab_filetree_live_deltas`: `laboratories create`
//!   + `laboratories attach`; the [`FileTree`] client connects BEFORE
//!   the container ever starts, so everything it sees arrives as LIVE
//!   deltas (the connect-time synthesized snapshot is empty).
//! - `agent_embedded_lab_filetree_snapshot`: the lab rides the agent
//!   definition's `laboratories` field (derived `oai-agent-…` id,
//!   discovered via `laboratories list`); the client connects AFTER
//!   the writes, so the connect-time synthesized snapshot must already
//!   carry them — the daemon-state path, no live delta needed.

mod cli_test_util;

use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::message::RequestMessage;
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::spawn::{
    Path as SpawnPath, Request as SpawnReq, RequestDangerousAdvanced,
    ResponseItem as SpawnItem,
};
use objectiveai_sdk::cli::command::agents::tags::apply::{
    Path as ApplyPath, Request as ApplyReq, Response as ApplyResp, Target as ApplyTarget,
};
use objectiveai_sdk::cli::command::laboratories::attach::{
    Path as AttachPath, Request as AttachReq, Response as AttachResp,
};
use objectiveai_sdk::cli::command::laboratories::create::{
    Kind, Path as CreatePath, Request as CreateReq, Response as CreateResp,
};
use objectiveai_sdk::cli::command::laboratories::list::{
    Path as ListPath, Request as ListReq, ResponseItem as ListItem,
};
use objectiveai_sdk::laboratories::filetree::{FileTree, FileTreeNode};
use serde_json::json;

/// Poll `$cond` (an `await`-ing bool) until true or a 180s deadline —
/// the listener tests' idiom. The hang-preventing executor's watchdog
/// only guards active CLI commands; filetree waits carry their own
/// bound here.
macro_rules! wait_for {
    ($desc:expr, $cond:expr) => {{
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        loop {
            if $cond {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {}",
                $desc
            );
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }};
}

/// The split base image — `docker.io/library/bash:latest` (the lab
/// actually boots and runs bash, so busybox won't do).
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

type Exec = cli_test_util::HangPreventingBinaryCommandExecutor;

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// Create a laboratory with `cwd` (the host's filetree pump watches
/// the lab's cwd, so the tests create in — and assert on — `/tmp`,
/// which exists in the base image).
async fn create_lab(executor: &Exec, id: &str, cwd: &str) {
    let created: CreateResp = cli_test_util::execute_one(
        executor,
        CreateReq {
            path_type: CreatePath::LaboratoriesCreate,
            kind: Kind::Client,
            id: id.to_string(),
            image: base_image(),
            mounts: Vec::new(),
            env: Vec::new(),
            cwd: cwd.to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(created.id, id);
}

/// Apply `tag` carrying `agent_json` (a GROUPED script agent).
async fn apply_tag(executor: &Exec, tag: &str, agent_json: serde_json::Value) {
    let agent_spec =
        serde_json::from_value::<InlineAgentBaseWithFallbacksOrRemoteCommitOptional>(agent_json)
            .expect("script agent spec deserializes");
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

/// Attach `lab` to `tag` (local machine, own state).
async fn attach_to_tag(executor: &Exec, tag: &str, lab: &str) {
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

/// Spawn the tag's agent with `message` and block until the instance
/// fully settles (every row consumed — the script's tool calls have
/// landed by then).
async fn spawn_and_wait(executor: &Exec, tag: &str, message: &str) {
    let items: Vec<SpawnItem> = cli_test_util::collect_stream(
        executor,
        SpawnReq {
            path_type: SpawnPath::AgentsSpawn,
            message: RequestMessage::Simple(message.to_string()),
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
    cli_test_util::wait_for_agent(executor, &aih).await;
}

/// The proxy-prefixed `Bash` tool name of an ATTACHED laboratory —
/// deterministic from the lab's composite identity.
fn bash_tool_name(lab_id: &str) -> String {
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
}

/// A one-turn script agent (`upstream: "script"`, python-on-client):
/// branches on the incoming message — "remove" ⇒ delete the marker,
/// else create the marker plus a nested `ft-dir/inner.txt` — and calls
/// the KNOWN attached-lab `Bash` tool. The trailing list literal is
/// the script's `OutputMessage[]`.
fn attached_script_agent(tool_name: &str, marker: &str) -> serde_json::Value {
    let python = format!(
        r#"import json
text = json.dumps(input)
if "remove" in text:
    cmd = "rm /tmp/{marker}"
else:
    cmd = "mkdir -p /tmp/ft-dir && printf hello > /tmp/ft-dir/inner.txt && printf embedded > /tmp/{marker}"
params = json.dumps({{"name": {tool_name:?}, "arguments": {{"command": cmd}}}})
objectiveai.execute(["agents", "mcp", "tools", "call", "--params", params])
[{{"role": "assistant", "content": "done"}}]"#
    );
    json!({
        "upstream": "script",
        "output_mode": "instruction",
        "instruction": "done",
        "type": "python",
        "python": python,
    })
}

/// A script agent with an EMBEDDED laboratory (the definition's
/// `laboratories` field — derived `oai-agent-…` id, created by the
/// conduit at MCP-initialize). The python cannot know the derived
/// server name up front, so it DISCOVERS the lab's `Bash` tool via
/// `agents mcp tools list` before calling it. Same message branching
/// as the attached variant — one spec, both phases, one derived id.
fn embedded_script_agent(marker: &str) -> serde_json::Value {
    let python = format!(
        r#"import json
text = json.dumps(input)
if "remove" in text:
    cmd = "rm /tmp/{marker}"
else:
    cmd = "printf embedded > /tmp/{marker}"
found = []
for item in objectiveai.execute(["agents", "mcp", "tools", "list", "--params", "{{}}"]):
    for tool in item.get("tools") or []:
        if tool["name"].endswith("_Bash"):
            found.append(tool["name"])
params = json.dumps({{"name": found[0], "arguments": {{"command": cmd}}}})
objectiveai.execute(["agents", "mcp", "tools", "call", "--params", params])
[{{"role": "assistant", "content": "done"}}]"#
    );
    json!({
        "upstream": "script",
        "output_mode": "instruction",
        "instruction": "done",
        "type": "python",
        "python": python,
        "laboratories": [
            {
                // The registry image's WIRE shape: the pin is flattened
                // (`tag` XOR `digest` at the top level), never nested.
                "image": {
                    "registry": "docker.io",
                    "name": "library/bash",
                    "tag": "latest"
                },
                "cwd": "/tmp"
            }
        ]
    })
}

/// The node named `name` among `nodes`, if any.
fn find<'a>(nodes: &'a [FileTreeNode], name: &str) -> Option<&'a FileTreeNode> {
    nodes.iter().find(|n| n.name() == name)
}

/// A directory node's children (`None` for files/symlinks).
fn dir_children(node: &FileTreeNode) -> Option<&[FileTreeNode]> {
    match node {
        FileTreeNode::Directory { children, .. } => Some(children),
        _ => None,
    }
}

/// Create + attach: the `FileTree` daemon client connects BEFORE the
/// container's first start, so the whole tree arrives as LIVE deltas
/// (lab boots → host pump opens → snapshot + writes push through the
/// daemon to the already-open SSE stream); a second spawn's `rm` then
/// lands as a removal delta on the same connection.
#[tokio::test(flavor = "multi_thread")]
async fn created_attached_lab_filetree_live_deltas() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;
    let state = cli_test_util::test_state_name();
    let addr = cli_test_util::daemon_address(&executor, &state).await;

    let n = nanos();
    let lab = format!("ft-{n}");
    let tag = format!("ft-tag-{n}");
    let marker = format!("ft-marker-{n}.txt");
    create_lab(&executor, &lab, "/tmp").await;

    // Connect while the container has never started: the synthesized
    // connect-time snapshot is empty, and everything below must reach
    // this client as pushed deltas.
    let tree = FileTree::daemon(&addr, &lab)
        .connect()
        .await
        .expect("connect /laboratories/{id}/filetree");

    apply_tag(&executor, &tag, attached_script_agent(&bash_tool_name(&lab), &marker)).await;
    attach_to_tag(&executor, &tag, &lab).await;
    spawn_and_wait(&executor, &tag, "create the files").await;

    wait_for!("marker + ft-dir/inner.txt as live deltas", {
        let nodes = tree.tree().await;
        let marker_ok = matches!(find(&nodes, &marker), Some(FileTreeNode::File { .. }));
        // The nested write proves recursive delivery: a Directory node
        // carrying its child File (5 bytes — "hello").
        let inner_ok = find(&nodes, "ft-dir")
            .and_then(dir_children)
            .and_then(|children| find(children, "inner.txt"))
            .is_some_and(|inner| {
                matches!(inner, FileTreeNode::File { size: Some(5), .. })
            });
        marker_ok && inner_ok
    });

    // Phase 2 — the SAME agent spec, message-branched to `rm`: the
    // marker vanishes from the live tree, the directory stays.
    spawn_and_wait(&executor, &tag, "remove the marker").await;
    wait_for!("marker removed, ft-dir kept", {
        let nodes = tree.tree().await;
        find(&nodes, &marker).is_none() && find(&nodes, "ft-dir").is_some()
    });
}

/// Agent-embedded laboratory: the lab exists only because the agent's
/// definition carries it (derived `oai-agent-…` id). The client
/// connects AFTER the script's writes, so the connect-time synthesized
/// snapshot — the daemon's materialized state — must already carry
/// them; a second spawn (same spec ⇒ same derived lab) then removes
/// the marker as a live delta.
#[tokio::test(flavor = "multi_thread")]
async fn agent_embedded_lab_filetree_snapshot() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;
    let state = cli_test_util::test_state_name();
    let addr = cli_test_util::daemon_address(&executor, &state).await;

    let n = nanos();
    let tag = format!("ft-embed-tag-{n}");
    let marker = format!("ft-embed-marker-{n}.txt");

    apply_tag(&executor, &tag, embedded_script_agent(&marker)).await;
    spawn_and_wait(&executor, &tag, "create the files").await;

    // Discover the derived laboratory — this state is isolated, so the
    // one `oai-agent-…` id is ours (its record carries the provenance).
    let mut lab_id = String::new();
    wait_for!("the derived oai-agent- laboratory on `laboratories list`", {
        let items: Vec<ListItem> = cli_test_util::collect_stream(
            &executor,
            ListReq {
                path_type: ListPath::LaboratoriesList,
                kind: Kind::Client,
                base: Default::default(),
            },
        )
        .await;
        match items.iter().find(|item| {
            item.id
                .starts_with(objectiveai_sdk::agent::AGENT_LABORATORY_ID_PREFIX)
        }) {
            Some(item) => {
                assert!(
                    item.agent_full_id.is_some(),
                    "derived laboratory must carry its agent provenance"
                );
                lab_id = item.id.clone();
                true
            }
            None => false,
        }
    });

    // Connect after the fact: the write happened before this client
    // existed, so it MUST come from the daemon's materialized snapshot.
    let tree = FileTree::daemon(&addr, &lab_id)
        .connect()
        .await
        .expect("connect /laboratories/{id}/filetree");
    wait_for!("marker in the connect-time snapshot", {
        let nodes = tree.tree().await;
        matches!(find(&nodes, &marker), Some(FileTreeNode::File { .. }))
    });

    // Phase 2 — same spec, so the message-branched `rm` runs in the
    // SAME derived laboratory; the removal reaches the open stream.
    spawn_and_wait(&executor, &tag, "remove the marker").await;
    wait_for!("marker removed from the embedded lab's tree", {
        find(&tree.tree().await, &marker).is_none()
    });
}
