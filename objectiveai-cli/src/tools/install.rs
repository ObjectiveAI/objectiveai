//! `tools install` — no install action; emits an INSTRUCTIONS.md
//! telling the agent how to author a tool locally by hand.

use objectiveai_sdk::cli::output::{Handle, Notification, Output};

#[derive(serde::Serialize)]
struct Instructions {
    instructions: String,
}

pub(super) async fn emit_instructions(handle: &Handle) -> Result<(), crate::error::Error> {
    let instructions = include_str!(
        "../../assets/tools/install/filesystem/INSTRUCTIONS.md"
    )
    .to_string();
    Output::<Instructions>::Notification(Notification {
        agent_id: None,
        value: Instructions { instructions },
    })
    .emit(handle)
    .await;
    Ok(())
}
