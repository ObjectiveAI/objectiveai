//! The provider's contribution: its id as the config's selection,
//! its credentials as the secrets the plan declares, its endpoint
//! facts as environment.

use super::{Plan, PrepareError, Secret, Target, VERTEX_FILE};
use super::HERMES_HOME;
use crate::agent::Provider;
use crate::agent::provider::bedrock;

/// Add the provider to the plan.
///
/// `model.provider` is the variant's own marker, serialized — the
/// SDK's kebab-case ids ARE Hermes's provider ids, so no table maps
/// one to the other. Then each variant's mechanism, as the agent's
/// provider docs state it: the key-only majority declare one secret
/// bound for the environment, under the variable's own name; copilot
/// its token the same way; azure-foundry and zai a key beside a
/// plain endpoint variable; custom its endpoint and, when asked, the
/// one secret bound for `model.api_key` (Hermes reads a custom key
/// from the config, not the environment); bedrock the name set its
/// auth mode fixes; vertex the document bound for the service-account
/// file; opencode-free nothing; and the four OAuth-state providers
/// declare their rotating document — three as `auth.json` entries,
/// qwen as the CLI's token file (its `providers.qwen-oauth` marker is
/// a setup-wizard artifact the gateway never reads, and is not
/// written).
pub fn apply(provider: &Provider, plan: &mut Plan) -> Result<(), PrepareError> {
    plan.provider = serde_json::to_value(provider)?
        .get("provider")
        .and_then(|marker| marker.as_str())
        .map(str::to_string)
        .unwrap_or_default();

    match provider {
        Provider::Actual(_) => env(plan, "ACTUAL_API_KEY"),
        Provider::AiGateway(_) => env(plan, "AI_GATEWAY_API_KEY"),
        Provider::Alibaba(_) => env(plan, "DASHSCOPE_API_KEY"),
        Provider::AlibabaCodingPlan(_) => env(plan, "ALIBABA_CODING_PLAN_API_KEY"),
        Provider::Anthropic(_) => env(plan, "ANTHROPIC_API_KEY"),
        Provider::Arcee(_) => env(plan, "ARCEEAI_API_KEY"),
        Provider::AzureFoundry(p) => {
            env(plan, "AZURE_FOUNDRY_API_KEY")?;
            plan.set("AZURE_FOUNDRY_BASE_URL", p.base_url.clone())
        }
        Provider::Bedrock(p) => {
            match p.auth {
                bedrock::Auth::BearerToken => {
                    env(plan, "AWS_BEARER_TOKEN_BEDROCK")?;
                }
                bedrock::Auth::AccessKey => {
                    env(plan, "AWS_ACCESS_KEY_ID")?;
                    env(plan, "AWS_SECRET_ACCESS_KEY")?;
                }
                bedrock::Auth::SessionToken => {
                    env(plan, "AWS_ACCESS_KEY_ID")?;
                    env(plan, "AWS_SECRET_ACCESS_KEY")?;
                    env(plan, "AWS_SESSION_TOKEN")?;
                }
            }
            if let Some(region) = &p.region {
                plan.set("AWS_REGION", region.clone())?;
            }
            Ok(())
        }
        Provider::Commandcode(_) => env(plan, "COMMANDCODE_API_KEY"),
        Provider::CommandcodeAnthropic(_) => env(plan, "COMMANDCODE_API_KEY"),
        Provider::Copilot(_) => env(plan, "COPILOT_GITHUB_TOKEN"),
        // The URL twice: the env var is what Hermes resolves first,
        // and the config key is what unlocks `model.api_key` — a key
        // is only read beside a configured base URL.
        Provider::Custom(p) => {
            plan.set("CUSTOM_BASE_URL", p.base_url.clone())?;
            plan.base_url = Some(p.base_url.clone());
            if p.api_key {
                plan.declare(Secret {
                    name: "CUSTOM_API_KEY",
                    target: Target::ModelApiKey,
                })?;
            }
            Ok(())
        }
        Provider::Deepinfra(_) => env(plan, "DEEPINFRA_API_KEY"),
        Provider::Deepseek(_) => env(plan, "DEEPSEEK_API_KEY"),
        Provider::Fireworks(_) => env(plan, "FIREWORKS_API_KEY"),
        // GEMINI_API_KEY, never GOOGLE_API_KEY: Hermes checks the
        // latter first, and it must not be set.
        Provider::Gemini(_) => env(plan, "GEMINI_API_KEY"),
        Provider::Gmi(_) => env(plan, "GMI_API_KEY"),
        Provider::Huggingface(_) => env(plan, "HF_TOKEN"),
        Provider::Kilocode(_) => env(plan, "KILOCODE_API_KEY"),
        Provider::KimiCoding(_) => env(plan, "KIMI_API_KEY"),
        Provider::KimiCodingCn(_) => env(plan, "KIMI_CN_API_KEY"),
        Provider::MetaAi(_) => env(plan, "MODEL_API_KEY"),
        Provider::Minimax(_) => env(plan, "MINIMAX_API_KEY"),
        Provider::MinimaxCn(_) => env(plan, "MINIMAX_CN_API_KEY"),
        Provider::MinimaxOauth(_) => plan.declare(Secret {
            name: "MINIMAX_OAUTH",
            target: Target::AuthEntry("minimax-oauth"),
        }),
        Provider::NebiusTokenFactory(_) => env(plan, "NEBIUS_API_KEY"),
        Provider::Nous(_) => plan.declare(Secret {
            name: "NOUS_OAUTH",
            target: Target::AuthEntry("nous"),
        }),
        Provider::Novita(_) => env(plan, "NOVITA_API_KEY"),
        Provider::Nvidia(_) => env(plan, "NVIDIA_API_KEY"),
        Provider::OllamaCloud(_) => env(plan, "OLLAMA_API_KEY"),
        Provider::OpenaiCodex(_) => plan.declare(Secret {
            name: "OPENAI_CODEX_OAUTH",
            target: Target::AuthEntry("openai-codex"),
        }),
        // Keyless: Hermes short-circuits it to an anonymous
        // placeholder before any credential check.
        Provider::OpencodeFree(_) => Ok(()),
        Provider::OpencodeGo(_) => env(plan, "OPENCODE_GO_API_KEY"),
        Provider::OpencodeZen(_) => env(plan, "OPENCODE_ZEN_API_KEY"),
        Provider::Openrouter(_) => env(plan, "OPENROUTER_API_KEY"),
        Provider::QwenOauth(_) => plan.declare(Secret {
            name: "QWEN_OAUTH",
            target: Target::QwenCreds,
        }),
        Provider::Router(_) => env(plan, "RAMP_ROUTER_API_KEY"),
        Provider::Stepfun(_) => env(plan, "STEPFUN_API_KEY"),
        Provider::Upstage(_) => env(plan, "UPSTAGE_API_KEY"),
        Provider::Vertex(p) => {
            plan.declare(Secret {
                name: "VERTEX_SERVICE_ACCOUNT_JSON",
                target: Target::VertexFile,
            })?;
            plan.set(
                "VERTEX_CREDENTIALS_PATH",
                format!("{HERMES_HOME}/{VERTEX_FILE}"),
            )?;
            if let Some(project_id) = &p.project_id {
                plan.set("VERTEX_PROJECT_ID", project_id.clone())?;
            }
            if let Some(region) = &p.region {
                plan.set("VERTEX_REGION", region.clone())?;
            }
            Ok(())
        }
        Provider::Xai(_) => env(plan, "XAI_API_KEY"),
        Provider::Xiaomi(_) => env(plan, "XIAOMI_API_KEY"),
        Provider::Zai(p) => {
            env(plan, "GLM_API_KEY")?;
            if let Some(base_url) = &p.base_url {
                plan.set("GLM_BASE_URL", base_url.clone())?;
            }
            Ok(())
        }
    }
}

/// Declare one secret bound for the gateway's environment, under
/// its own name.
fn env(plan: &mut Plan, name: &'static str) -> Result<(), PrepareError> {
    plan.declare(Secret {
        name,
        target: Target::Env,
    })
}
