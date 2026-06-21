//! Smoke test: every `request-schema` and `response-schema` cli command
//! runs cleanly.
//!
//! The set of commands is discovered DYNAMICALLY from the cli's own clap
//! command tree — `objectiveai_sdk::cli::command::Command`, the top-level
//! `clap::Parser` — so a newly-added leaf is covered with no hard-coded
//! list and no per-command edits here. This is the authoritative source:
//! it's the parser the binary itself uses, so the enumeration can't drift
//! from what the cli actually accepts (unlike reconstructing argv from the
//! json-schema folder or the source tree, which would have to re-derive
//! the kebab-casing and break on any custom clap name).
//!
//! For each discovered path we rebuild the typed aggregate `Request` via
//! the SDK's `parse_request` (argv -> `Request`) — no hand-written struct
//! literals — then dispatch it through the on-disk cli with `BinaryExecutor`
//! (which ships the request as `--request <json>`) and assert the stream
//! yields no error.
//!
//! These commands only serialize a JSON Schema and exit — no network, db,
//! or config — so they are hermetic and need none of the api/db/state
//! harness. Each invocation execs the compiled on-disk cli binary directly
//! (no per-call build step), and serializing a schema is trivial and
//! short-lived, so every command runs fully in parallel.

mod cli_test_util;

use clap::CommandFactory;
use futures::StreamExt;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;
use objectiveai_sdk::cli::command::{
    parse_request, Command as CliCommand, CommandExecutor, Request, ResponseItem,
};

/// Walk the cli's clap command tree and collect the full argv path of every
/// leaf whose terminal subcommand is `terminal` (`"request-schema"` or
/// `"response-schema"`). Paths come back WITHOUT the `objectiveai`
/// program-name prefix — exactly the shape `parse_request` accepts.
fn schema_command_paths(terminal: &str) -> Vec<Vec<String>> {
    fn walk(cmd: &clap::Command, prefix: &[String], terminal: &str, out: &mut Vec<Vec<String>>) {
        for sub in cmd.get_subcommands() {
            let name = sub.get_name().to_string();
            let mut path = prefix.to_vec();
            path.push(name.clone());
            if name == terminal {
                out.push(path.clone());
            }
            // Recurse anyway: request-schema/response-schema only appear as
            // leaves today, but a name match at any depth is still collected
            // with its full path, so nesting changes can't silently drop one.
            walk(sub, &path, terminal, out);
        }
    }
    let root = CliCommand::command();
    let mut out = Vec::new();
    walk(&root, &[], terminal, &mut out);
    out
}

/// Run every `terminal`-named schema command and assert none errors. A
/// command "fails" if `parse_request` rejects its argv, the executor can't
/// dispatch it, any streamed line decodes to a `cli::Error`, or it produces
/// no output at all. ALL failures are collected before asserting, so a
/// single run reports every broken command rather than just the first.
async fn assert_all_schema_commands_succeed(terminal: &str) {
    let paths = schema_command_paths(terminal);
    assert!(
        !paths.is_empty(),
        "no `{terminal}` commands discovered from the clap command tree — \
         the walk is broken (or the cli exposes no schema commands)"
    );

    // One shared executor over the compiled on-disk cli binary. Schema
    // commands are hermetic, so a bare BinaryExecutor is enough. OBJECTIVEAI_DIR
    // is pinned to the repo's shared test root so the cli never touches the
    // developer's real `~/.objectiveai`; OBJECTIVEAI_STATE matches the suite.
    let executor = BinaryExecutor::from_path(cli_test_util::cli_binary())
        .env(
            "OBJECTIVEAI_DIR",
            cli_test_util::objectiveai_dir()
                .to_string_lossy()
                .into_owned(),
        )
        .env("OBJECTIVEAI_STATE", cli_test_util::test_state_name());

    // All commands in flight at once — hermetic, trivial, short-lived.
    let executor = &executor;
    let results = futures::future::join_all(paths.into_iter().map(|argv| async move {
        let display = argv.join(" ");
        let request = parse_request(&argv)
            .map_err(|e| format!("`{display}`: parse_request failed: {e}"))?;
        let mut response = executor
            .execute::<Request, ResponseItem>(request, None)
            .await
            .map_err(|e| format!("`{display}`: executor dispatch failed: {e:?}"))?;
        let mut count = 0usize;
        while let Some(item) = response.next().await {
            item.map_err(|e| format!("`{display}`: stream error: {e:?}"))?;
            count += 1;
        }
        if count == 0 {
            return Err(format!("`{display}`: produced no output"));
        }
        Ok::<(), String>(())
    }))
    .await;
    let failures: Vec<String> = results.into_iter().filter_map(Result::err).collect();

    assert!(
        failures.is_empty(),
        "{} `{terminal}` command(s) failed:\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

#[tokio::test]
async fn every_request_schema_command_succeeds() {
    assert_all_schema_commands_succeed("request-schema").await;
}

#[tokio::test]
async fn every_response_schema_command_succeeds() {
    assert_all_schema_commands_succeed("response-schema").await;
}
