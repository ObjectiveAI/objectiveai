//! E2E: `laboratories create` + `laboratories list`.
//!
//! Creates a laboratory and confirms `list` reads its spec (id, image,
//! env, cwd) back — all through the CLI laboratories commands.

mod cli_test_util;

use objectiveai_sdk::cli::command::laboratories::create::{
    EnvVar, Kind, Path as CreatePath, Request as CreateReq, Response as CreateResp,
};
use objectiveai_sdk::cli::command::laboratories::list::{
    Path as ListPath, Request as ListReq, ResponseItem as ListItem,
};

/// Expect a duplicate `create` for `id` to fail with "already exists".
async fn expect_create_err(executor: &cli_test_util::HangPreventingBinaryCommandExecutor, id: &str) {
    use futures::StreamExt;
    use objectiveai_sdk::cli::command::CommandExecutor;
    let stream = executor
        .execute::<CreateReq, CreateResp>(
            CreateReq {
                path_type: CreatePath::LaboratoriesCreate,
                kind: Kind::Client,
                id: id.to_string(),
                image: base_image(),
                mounts: Vec::new(),
                env: Vec::new(),
                cwd: "/work".to_string(),
                machine: None,
                machine_state: None,
                base: Default::default(),
            },
            None,
        )
        .await
        .expect("cli execute must start");
    let mut stream = std::pin::pin!(stream);
    let mut saw = false;
    while let Some(item) = stream.next().await {
        if let Err(e) = item {
            if format!("{e:?}").contains("already exists") {
                saw = true;
            }
        }
    }
    assert!(saw, "duplicate create must error with 'already exists'");
}

/// A minimal, widely-available base image for the laboratory.
/// The split base image every lab in this file uses —
/// `docker.io/library/busybox:latest`, as parts (a joined reference string
/// is unrepresentable in the API).
fn base_image() -> objectiveai_sdk::laboratories::LaboratoryImage {
    objectiveai_sdk::laboratories::LaboratoryImage {
        registry: "docker.io".to_string(),
        name: "library/busybox".to_string(),
        pin: objectiveai_sdk::laboratories::LaboratoryImagePin::Tag(
            "latest".to_string(),
        ),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn create_then_list_round_trips_the_spec() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // Unique id (state is shared across runs; no `laboratories delete`).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = format!("e2e-lab-{nanos}");

    let created: CreateResp = cli_test_util::execute_one(
        &executor,
        CreateReq {
            path_type: CreatePath::LaboratoriesCreate,
            kind: Kind::Client,
            id: id.clone(),
            image: base_image(),
            mounts: Vec::new(),
            env: vec![EnvVar {
                key: "FOO".to_string(),
                value: "bar".to_string(),
            }],
            cwd: "/work".to_string(),
            machine: None,
            machine_state: None,
            base: Default::default(),
        },
    )
    .await;
    assert_eq!(created.id, id);
    assert_eq!(created.cwd, "/work");

    let labs: Vec<ListItem> = cli_test_util::collect_stream(
        &executor,
        ListReq {
            path_type: ListPath::LaboratoriesList,
            kind: Kind::Client,
            base: Default::default(),
        },
    )
    .await;

    let found = labs
        .iter()
        .find(|l| l.id == id)
        .unwrap_or_else(|| panic!("created lab {id} not in list: {:?}", labs.iter().map(|l| &l.id).collect::<Vec<_>>()));
    assert_eq!(found.image, base_image());
    assert_eq!(found.cwd, "/work");
    // A laboratory created with no --machine lands on THIS machine's
    // host — the item's machine identity must match the one the
    // daemon computes for this machine (there is no local-vs-remote;
    // machine identity is the only provenance).
    let local_machine =
        objectiveai_sdk::machine::machine_id(&cli_test_util::objectiveai_dir());
    assert_eq!(
        found.machine.as_ref().map(|m| m.id.as_str()),
        Some(local_machine.as_str()),
        "a laboratory created on this machine + state must be served by this machine's host"
    );

    // A second create for the same id must fail loudly.
    expect_create_err(&executor, &id).await;
    assert!(
        found.env.iter().any(|e| e.key == "FOO" && e.value == "bar"),
        "env not round-tripped: {:?}",
        found.env.iter().map(|e| (&e.key, &e.value)).collect::<Vec<_>>()
    );
}
