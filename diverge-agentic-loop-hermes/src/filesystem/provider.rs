//! The provider's contribution: its credential as environment, its
//! id as the config's selection, its login as a vault document.

use diverge_sdk::shared::containers::vault::keys;

use super::{Document, Plan, PrepareError, Target, VERTEX_FILE};
use super::HERMES_HOME;
use crate::agent::Provider;
use crate::agent::provider::bedrock;

/// Add the provider to the plan.
///
/// `model.provider` is the variant's own marker, serialized — the
/// SDK's kebab-case ids ARE Hermes's provider ids, so no table maps
/// one to the other. Then each variant's mechanism, as the SDK's
/// provider docs state it: the key-only majority set one variable;
/// copilot, azure-foundry, custom, zai, bedrock and vertex set their
/// several; opencode-free sets nothing; and the four OAuth-state
/// providers name their vault document — three as `auth.json`
/// entries, qwen as the CLI's token file (its `providers.qwen-oauth`
/// marker is a setup-wizard artifact the gateway never reads, and is
/// not written).
pub fn apply(provider: &Provider, plan: &mut Plan) -> Result<(), PrepareError> {
    plan.provider = serde_json::to_value(provider)?
        .get("provider")
        .and_then(|marker| marker.as_str())
        .map(str::to_string)
        .unwrap_or_default();

    match provider {
        Provider::Actual(p) => plan.set("ACTUAL_API_KEY", p.api_key.clone()),
        Provider::AiGateway(p) => {
            plan.set("AI_GATEWAY_API_KEY", p.api_key.clone())
        }
        Provider::Alibaba(p) => plan.set("DASHSCOPE_API_KEY", p.api_key.clone()),
        Provider::AlibabaCodingPlan(p) => {
            plan.set("ALIBABA_CODING_PLAN_API_KEY", p.api_key.clone())
        }
        Provider::Anthropic(p) => {
            plan.set("ANTHROPIC_API_KEY", p.api_key.clone())
        }
        Provider::Arcee(p) => plan.set("ARCEEAI_API_KEY", p.api_key.clone()),
        Provider::AzureFoundry(p) => {
            plan.set("AZURE_FOUNDRY_API_KEY", p.api_key.clone())?;
            plan.set("AZURE_FOUNDRY_BASE_URL", p.base_url.clone())
        }
        Provider::Bedrock(p) => {
            match &p.auth {
                bedrock::Auth::BearerToken { bearer_token } => {
                    plan.set("AWS_BEARER_TOKEN_BEDROCK", bearer_token.clone())?;
                }
                bedrock::Auth::AccessKey {
                    access_key_id,
                    secret_access_key,
                    session_token,
                } => {
                    plan.set("AWS_ACCESS_KEY_ID", access_key_id.clone())?;
                    plan.set(
                        "AWS_SECRET_ACCESS_KEY",
                        secret_access_key.clone(),
                    )?;
                    if let Some(session_token) = session_token {
                        plan.set("AWS_SESSION_TOKEN", session_token.clone())?;
                    }
                }
            }
            if let Some(region) = &p.region {
                plan.set("AWS_REGION", region.clone())?;
            }
            Ok(())
        }
        Provider::Commandcode(p) => {
            plan.set("COMMANDCODE_API_KEY", p.api_key.clone())
        }
        Provider::CommandcodeAnthropic(p) => {
            plan.set("COMMANDCODE_API_KEY", p.api_key.clone())
        }
        Provider::Copilot(p) => {
            plan.set("COPILOT_GITHUB_TOKEN", p.github_token.clone())
        }
        // The URL twice: the env var is what Hermes resolves first,
        // and the config key is what unlocks `model.api_key` — a key
        // is only read beside a configured base URL.
        Provider::Custom(p) => {
            plan.set("CUSTOM_BASE_URL", p.base_url.clone())?;
            plan.base_url = Some(p.base_url.clone());
            plan.api_key = p.api_key.clone();
            Ok(())
        }
        Provider::Deepinfra(p) => {
            plan.set("DEEPINFRA_API_KEY", p.api_key.clone())
        }
        Provider::Deepseek(p) => plan.set("DEEPSEEK_API_KEY", p.api_key.clone()),
        Provider::Fireworks(p) => {
            plan.set("FIREWORKS_API_KEY", p.api_key.clone())
        }
        // GEMINI_API_KEY, never GOOGLE_API_KEY: Hermes checks the
        // latter first, and it must not be set.
        Provider::Gemini(p) => plan.set("GEMINI_API_KEY", p.api_key.clone()),
        Provider::Gmi(p) => plan.set("GMI_API_KEY", p.api_key.clone()),
        Provider::Huggingface(p) => plan.set("HF_TOKEN", p.api_key.clone()),
        Provider::Kilocode(p) => plan.set("KILOCODE_API_KEY", p.api_key.clone()),
        Provider::KimiCoding(p) => plan.set("KIMI_API_KEY", p.api_key.clone()),
        Provider::KimiCodingCn(p) => {
            plan.set("KIMI_CN_API_KEY", p.api_key.clone())
        }
        Provider::MetaAi(p) => plan.set("MODEL_API_KEY", p.api_key.clone()),
        Provider::Minimax(p) => plan.set("MINIMAX_API_KEY", p.api_key.clone()),
        Provider::MinimaxCn(p) => {
            plan.set("MINIMAX_CN_API_KEY", p.api_key.clone())
        }
        Provider::MinimaxOauth(_) => {
            plan.documents.push(Document {
                key: keys::MINIMAX_OAUTH,
                target: Target::AuthEntry("minimax-oauth"),
            });
            Ok(())
        }
        Provider::NebiusTokenFactory(p) => {
            plan.set("NEBIUS_API_KEY", p.api_key.clone())
        }
        Provider::Nous(_) => {
            plan.documents.push(Document {
                key: keys::NOUS_OAUTH,
                target: Target::AuthEntry("nous"),
            });
            Ok(())
        }
        Provider::Novita(p) => plan.set("NOVITA_API_KEY", p.api_key.clone()),
        Provider::Nvidia(p) => plan.set("NVIDIA_API_KEY", p.api_key.clone()),
        Provider::OllamaCloud(p) => plan.set("OLLAMA_API_KEY", p.api_key.clone()),
        Provider::OpenaiCodex(_) => {
            plan.documents.push(Document {
                key: keys::OPENAI_CODEX_OAUTH,
                target: Target::AuthEntry("openai-codex"),
            });
            Ok(())
        }
        // Keyless: Hermes short-circuits it to an anonymous
        // placeholder before any credential check.
        Provider::OpencodeFree(_) => Ok(()),
        Provider::OpencodeGo(p) => {
            plan.set("OPENCODE_GO_API_KEY", p.api_key.clone())
        }
        Provider::OpencodeZen(p) => {
            plan.set("OPENCODE_ZEN_API_KEY", p.api_key.clone())
        }
        Provider::Openrouter(p) => {
            plan.set("OPENROUTER_API_KEY", p.api_key.clone())
        }
        Provider::QwenOauth(_) => {
            plan.documents.push(Document {
                key: keys::QWEN_OAUTH,
                target: Target::QwenCreds,
            });
            Ok(())
        }
        Provider::Router(p) => plan.set("RAMP_ROUTER_API_KEY", p.api_key.clone()),
        Provider::Stepfun(p) => plan.set("STEPFUN_API_KEY", p.api_key.clone()),
        Provider::Upstage(p) => plan.set("UPSTAGE_API_KEY", p.api_key.clone()),
        Provider::Vertex(p) => {
            plan.vertex = Some(p.service_account_json.clone());
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
        Provider::Xai(p) => plan.set("XAI_API_KEY", p.api_key.clone()),
        Provider::Xiaomi(p) => plan.set("XIAOMI_API_KEY", p.api_key.clone()),
        Provider::Zai(p) => {
            plan.set("GLM_API_KEY", p.api_key.clone())?;
            if let Some(base_url) = &p.base_url {
                plan.set("GLM_BASE_URL", base_url.clone())?;
            }
            Ok(())
        }
    }
}
