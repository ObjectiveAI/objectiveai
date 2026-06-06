//! Shared mock data for Agents, Swarms, Functions, and Profiles.
//!
//! Provides embedded JSON fixtures used by the retrieval module.

/// Returns true if the name exists in any mock repository (agents, swarms, functions, profiles).
pub fn exists(name: &str) -> bool {
    AGENT_REPOSITORIES.contains(&name)
        || SWARM_REPOSITORIES.contains(&name)
        || FUNCTION_REPOSITORIES.contains(&name)
        || PROFILE_REPOSITORIES.contains(&name)
        || PROMPT_REPOSITORIES.contains(&name)
}

/// Returns a mock Agent by name.
pub fn get_agent(
    name: &str,
) -> Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks> {
    let json = get_agent_json(name)?;
    Some(serde_json::from_str(json).expect("invalid mock agent JSON"))
}

/// Returns mock Agent JSON by name.
fn get_agent_json(name: &str) -> Option<&'static str> {
    match name {
        "schema-logprobs" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/agents/schema-logprobs.json"))),
        "instruction" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/agents/instruction.json"))),
        "tool-call" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/agents/tool-call.json"))),
        "instruction-logprobs" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/agents/instruction-logprobs.json"))),
        "invention" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/agents/invention.json"))),
        "error-probability-50" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/agents/error-probability-50.json"))),
        "json-schema-10x-tools" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/agents/json-schema-10x-tools.json"))),
        _ => None,
    }
}

/// All mock Agent names.
const AGENT_REPOSITORIES: &[&str] = &["schema-logprobs", "instruction", "tool-call", "instruction-logprobs", "invention", "error-probability-50", "json-schema-10x-tools"];

/// Lists all mock Agents.
pub fn list_agents() -> objectiveai_sdk::agent::response::ListAgentResponse {
    objectiveai_sdk::agent::response::ListAgentResponse {
        data: AGENT_REPOSITORIES
            .iter()
            .map(|name| objectiveai_sdk::RemotePath::Mock {
                name: name.to_string(),
            })
            .collect(),
    }
}

/// Returns a mock Swarm by name.
pub fn get_swarm(
    name: &str,
) -> Option<objectiveai_sdk::swarm::RemoteSwarmBase> {
    let json = get_swarm_json(name)?;
    Some(serde_json::from_str(json).expect("invalid mock swarm JSON"))
}

/// Returns mock Swarm JSON by name.
fn get_swarm_json(name: &str) -> Option<&'static str> {
    match name {
        "schema-and-tool" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/swarms/schema-and-tool.json"))),
        "instruction-duo" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/swarms/instruction-duo.json"))),
        "twenty-agents-json-schema-10x-tools" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/swarms/twenty-agents-json-schema-10x-tools.json"))),
        _ => None,
    }
}

/// All mock Swarm names.
const SWARM_REPOSITORIES: &[&str] = &["schema-and-tool", "instruction-duo", "twenty-agents-json-schema-10x-tools"];

/// Lists all mock Swarms.
pub fn list_swarms() -> objectiveai_sdk::swarm::response::ListSwarmResponse {
    objectiveai_sdk::swarm::response::ListSwarmResponse {
        data: SWARM_REPOSITORIES
            .iter()
            .map(|name| objectiveai_sdk::RemotePath::Mock {
                name: name.to_string(),
            })
            .collect(),
    }
}

/// Returns a mock Function by name.
pub fn get_function(
    name: &str,
) -> Option<objectiveai_sdk::functions::FullRemoteFunction> {
    let json = get_function_json(name)?;
    Some(serde_json::from_str(json).expect("invalid mock function JSON"))
}

/// Returns mock Function JSON by repository name.
fn get_function_json(repository: &str) -> Option<&'static str> {
    match repository {
        "binary-classifier" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/binary-classifier.json"))),
        "spam-with-optional-sentiment" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/spam-with-optional-sentiment.json"))),
        "five-star-rating" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/five-star-rating.json"))),
        "item-ranker" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/item-ranker.json"))),
        "contextual-ranker" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/contextual-ranker.json"))),
        "email-importance" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/email-importance.json"))),
        "five-criteria-ranker" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/five-criteria-ranker.json"))),
        "strict-contextual-ranker" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/strict-contextual-ranker.json"))),
        "spam-importance-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/spam-importance-branch.json"))),
        "triple-classifier-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/triple-classifier-branch.json"))),
        "classifier-with-optional-sentiment" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/classifier-with-optional-sentiment.json"))),
        "dual-ranker-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/dual-ranker-branch.json"))),
        "mixed-scalar-vector-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/mixed-scalar-vector-branch.json"))),
        "ranker-with-optional-quality" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/ranker-with-optional-quality.json"))),
        "triple-ranker-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/triple-ranker-branch.json"))),
        "four-way-vector-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/four-way-vector-branch.json"))),
        "deep-optional-mixed-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/deep-optional-mixed-branch.json"))),
        "nested-scalar-super-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/nested-scalar-super-branch.json"))),
        "skipable-nested-scalar-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/skipable-nested-scalar-branch.json"))),
        "nested-vector-super-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/nested-vector-super-branch.json"))),
        "contextual-nested-vector-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/contextual-nested-vector-branch.json"))),
        "mapped-branch-with-votes" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/mapped-branch-with-votes.json"))),
        "mapped-branch-with-classifiers" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/mapped-branch-with-classifiers.json"))),
        "mapped-branch-mixed-tasks" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/mapped-branch-mixed-tasks.json"))),
        "dual-placeholder" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/dual-placeholder.json"))),
        "error-bad-output-field" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-bad-output-field.json"))),
        "error-scalar-out-of-range" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-scalar-out-of-range.json"))),
        "error-scalar-returns-vector" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-scalar-returns-vector.json"))),
        "error-vector-bad-sum" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-vector-bad-sum.json"))),
        "error-vector-returns-scalar" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-vector-returns-scalar.json"))),
        "error-nested-list-output" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-nested-list-output.json"))),
        "error-none-output" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-none-output.json"))),
        "error-missing-input-key" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-missing-input-key.json"))),
        "error-missing-sub-function" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-missing-sub-function.json"))),
        "error-wrong-sub-input" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-wrong-sub-input.json"))),
        "error-cycle-a" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-cycle-a.json"))),
        "error-cycle-b" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-cycle-b.json"))),
        "error-cycle-abc-a" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-cycle-abc-a.json"))),
        "error-cycle-abc-b" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-cycle-abc-b.json"))),
        "error-cycle-abc-c" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/error-cycle-abc-c.json"))),
        "tweet-scorer" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-scorer.json"))),
        "tweet-3cffCh20" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-3cffCh20.json"))),
        "tweet-3cffCh21" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-3cffCh21.json"))),
        "tweet-3cffCh22" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-3cffCh22.json"))),
        "tweet-ranker" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-ranker.json"))),
        "tweet-3YQMcu20" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-3YQMcu20.json"))),
        "tweet-3YQMcu21" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-3YQMcu21.json"))),
        "tweet-3YQMcu22" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/tweet-3YQMcu22.json"))),
        "twenty-agents-json-schema-10x-tools-vector" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/functions/twenty-agents-json-schema-10x-tools-vector.json"))),
        _ => None,
    }
}

/// Returns a mock invention state file by name and filename.
pub fn get_invention_state_file(name: &str, filename: &str) -> Option<&'static str> {
    macro_rules! inv {
        ($name:expr, $file:expr) => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/mock/functions/inventions/",
                $name, "/", $file,
            ))
        };
    }
    match (name, filename) {
        // inv-good-sl
        ("inv-good-sl", "parameters.json") => Some(inv!("inv-good-sl", "parameters.json")),
        ("inv-good-sl", "function.json") => Some(inv!("inv-good-sl", "function.json")),
        ("inv-good-sl", "input_schema.json") => Some(inv!("inv-good-sl", "input_schema.json")),
        ("inv-good-sl", "ESSAY_TASKS.md") => Some(inv!("inv-good-sl", "ESSAY_TASKS.md")),
        // inv-good-vl
        ("inv-good-vl", "parameters.json") => Some(inv!("inv-good-vl", "parameters.json")),
        ("inv-good-vl", "function.json") => Some(inv!("inv-good-vl", "function.json")),
        ("inv-good-vl", "input_schema.json") => Some(inv!("inv-good-vl", "input_schema.json")),
        ("inv-good-vl", "ESSAY.md") => Some(inv!("inv-good-vl", "ESSAY.md")),
        // inv-schema-only
        ("inv-schema-only", "parameters.json") => Some(inv!("inv-schema-only", "parameters.json")),
        ("inv-schema-only", "input_schema.json") => Some(inv!("inv-schema-only", "input_schema.json")),
        ("inv-schema-only", "ESSAY.md") => Some(inv!("inv-schema-only", "ESSAY.md")),
        _ => None,
    }
}

/// All mock Function repository names.
const FUNCTION_REPOSITORIES: &[&str] = &[
    "binary-classifier", "spam-with-optional-sentiment", "five-star-rating",
    "item-ranker", "contextual-ranker", "email-importance",
    "five-criteria-ranker", "strict-contextual-ranker",
    "spam-importance-branch", "triple-classifier-branch",
    "classifier-with-optional-sentiment", "dual-ranker-branch",
    "mixed-scalar-vector-branch", "ranker-with-optional-quality",
    "triple-ranker-branch", "four-way-vector-branch",
    "deep-optional-mixed-branch", "nested-scalar-super-branch",
    "skipable-nested-scalar-branch", "nested-vector-super-branch",
    "contextual-nested-vector-branch",
    "mapped-branch-with-votes", "mapped-branch-with-classifiers",
    "mapped-branch-mixed-tasks", "dual-placeholder",
    "error-bad-output-field", "error-scalar-out-of-range",
    "error-scalar-returns-vector", "error-vector-bad-sum",
    "error-vector-returns-scalar", "error-nested-list-output",
    "error-none-output", "error-missing-input-key",
    "error-missing-sub-function", "error-wrong-sub-input",
    "error-cycle-a", "error-cycle-b",
    "error-cycle-abc-a", "error-cycle-abc-b", "error-cycle-abc-c",
    "tweet-scorer",
    "tweet-3cffCh20", "tweet-3cffCh21", "tweet-3cffCh22",
    "tweet-ranker",
    "tweet-3YQMcu20", "tweet-3YQMcu21", "tweet-3YQMcu22",
    "twenty-agents-json-schema-10x-tools-vector",
];

/// Lists all mock Functions.
pub fn list_functions() -> objectiveai_sdk::functions::response::ListFunctionResponse {
    objectiveai_sdk::functions::response::ListFunctionResponse {
        data: FUNCTION_REPOSITORIES
            .iter()
            .map(|repo| objectiveai_sdk::RemotePath::Mock {
                name: repo.to_string(),
            })
            .collect(),
    }
}

/// Returns a mock Profile by name.
pub fn get_profile(
    name: &str,
) -> Option<objectiveai_sdk::functions::RemoteProfile> {
    let json = get_profile_json(name)?;
    Some(serde_json::from_str(json).expect("invalid mock profile JSON"))
}

/// Returns mock Profile JSON by repository name.
fn get_profile_json(repository: &str) -> Option<&'static str> {
    match repository {
        "solo-instruction" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/solo-instruction.json"))),
        "instruction-and-schema" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/instruction-and-schema.json"))),
        "triple-mode" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/triple-mode.json"))),
        "contextual-duo" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/contextual-duo.json"))),
        "schema-heavy-trio" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/schema-heavy-trio.json"))),
        "logprobs-and-tool" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/logprobs-and-tool.json"))),
        "schema-logprobs-solo" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/schema-logprobs-solo.json"))),
        "trio-with-error-agent" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/trio-with-error-agent.json"))),
        "tool-and-schema" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/tool-and-schema.json"))),
        "logprobs-duo" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/logprobs-duo.json"))),
        "schema-solo" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/schema-solo.json"))),
        "trio-with-error-instruction" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/trio-with-error-instruction.json"))),
        "high-logprobs-duo" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/high-logprobs-duo.json"))),
        "quad-with-error" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/quad-with-error.json"))),
        "max-logprobs-duo" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/max-logprobs-duo.json"))),
        "expanded-nested-scalar" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/expanded-nested-scalar.json"))),
        "mixed-nested-with-skip" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/mixed-nested-with-skip.json"))),
        "nested-vector-inline-remote" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/nested-vector-inline-remote.json"))),
        "deep-nested-vector" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/deep-nested-vector.json"))),
        "remote-swarm-mapped-branch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/remote-swarm-mapped-branch.json"))),
        "remote-swarm-classifiers" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/remote-swarm-classifiers.json"))),
        "baseline-auto" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/baseline-auto.json"))),
        "baseline-tasks" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/baseline-tasks.json"))),
        "two-task-tasks" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/two-task-tasks.json"))),
        "error-weights-length-mismatch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/error-weights-length-mismatch.json"))),
        "placeholder-and-remote-tasks" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/placeholder-and-remote-tasks.json"))),
        "error-dangling-swarm-ref" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/error-dangling-swarm-ref.json"))),
        "error-weight-count-mismatch" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/error-weight-count-mismatch.json"))),
        "dangling-and-valid-tasks" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/dangling-and-valid-tasks.json"))),
        "error-all-agents-fail" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/error-all-agents-fail.json"))),
        "twenty-agents-json-schema-10x-tools-profile" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/profiles/twenty-agents-json-schema-10x-tools-profile.json"))),
        _ => None,
    }
}

/// All mock Profile repository names.
const PROFILE_REPOSITORIES: &[&str] = &[
    "solo-instruction", "instruction-and-schema", "triple-mode",
    "contextual-duo", "schema-heavy-trio", "logprobs-and-tool",
    "schema-logprobs-solo", "trio-with-error-agent",
    "tool-and-schema", "logprobs-duo", "schema-solo",
    "trio-with-error-instruction", "high-logprobs-duo",
    "quad-with-error", "max-logprobs-duo",
    "expanded-nested-scalar", "mixed-nested-with-skip",
    "nested-vector-inline-remote", "deep-nested-vector",
    "remote-swarm-mapped-branch", "remote-swarm-classifiers",
    "baseline-auto", "baseline-tasks",
    "two-task-tasks", "error-weights-length-mismatch",
    "placeholder-and-remote-tasks", "error-dangling-swarm-ref",
    "error-weight-count-mismatch", "dangling-and-valid-tasks",
    "error-all-agents-fail",
    "twenty-agents-json-schema-10x-tools-profile",
];

/// Lists all mock Profiles.
pub fn list_profiles() -> objectiveai_sdk::functions::profiles::response::ListProfileResponse {
    objectiveai_sdk::functions::profiles::response::ListProfileResponse {
        data: PROFILE_REPOSITORIES
            .iter()
            .map(|repo| objectiveai_sdk::RemotePath::Mock {
                name: repo.to_string(),
            })
            .collect(),
    }
}

/// Returns a mock Prompt by name.
pub fn get_prompt(
    name: &str,
) -> Option<objectiveai_sdk::functions::inventions::prompts::RemotePrompt> {
    let json = get_prompt_json(name)?;
    Some(serde_json::from_str(json).expect("invalid mock prompt JSON"))
}

/// Returns mock Prompt JSON by repository name.
fn get_prompt_json(repository: &str) -> Option<&'static str> {
    match repository {
        "default" => Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mock/prompts/default.json"))),
        _ => None,
    }
}

/// All mock Prompt repository names.
const PROMPT_REPOSITORIES: &[&str] = &[
    "default",
];

/// Lists all mock Prompts.
pub fn list_prompts() -> objectiveai_sdk::functions::inventions::prompts::response::ListPromptResponse {
    objectiveai_sdk::functions::inventions::prompts::response::ListPromptResponse {
        data: PROMPT_REPOSITORIES
            .iter()
            .map(|repo| objectiveai_sdk::RemotePath::Mock {
                name: repo.to_string(),
            })
            .collect(),
    }
}
