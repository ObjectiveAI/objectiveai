//! The provider — one inference source's arguments.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The inference source, credentials and all.
///
/// Untagged, discriminated by each variant's own `provider` constant
/// — the same discipline the [`Agent`](super::super::super::Agent)
/// union uses for `upstream`. One variant per provider because the
/// argument sets genuinely differ: most are a single key, but
/// bedrock is AWS credentials, vertex is a service-account document,
/// azure-foundry and custom carry their endpoints — and each key
/// applies through a different mechanism the variant's own docs
/// spell out.
///
/// CLOSED at the pin, deliberately: user plugins can extend Hermes's
/// registry, but the container runs stock Hermes, so the bundled set
/// is the whole vocabulary — minus `copilot-acp`, whose auth is not
/// Hermes's to hold (see [the module](super)). Rotating-OAuth
/// providers carry their state as RESOURCES rather than plain
/// values; the module doc says what that means.
///
/// Hermes's own `auto` — provider detection from whatever
/// credentials happen to be present — is deliberately NOT here, and
/// under auth-as-arguments it is not even coherent: the request
/// carries exactly one provider's credentials, so there is nothing
/// to detect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Provider {
    /// See [`actual::Provider`](super::actual::Provider).
    Actual(super::actual::Provider),
    /// See [`ai_gateway::Provider`](super::ai_gateway::Provider).
    AiGateway(super::ai_gateway::Provider),
    /// See [`alibaba::Provider`](super::alibaba::Provider).
    Alibaba(super::alibaba::Provider),
    /// See [`alibaba_coding_plan::Provider`](super::alibaba_coding_plan::Provider).
    AlibabaCodingPlan(super::alibaba_coding_plan::Provider),
    /// See [`anthropic::Provider`](super::anthropic::Provider).
    Anthropic(super::anthropic::Provider),
    /// See [`arcee::Provider`](super::arcee::Provider).
    Arcee(super::arcee::Provider),
    /// See [`azure_foundry::Provider`](super::azure_foundry::Provider).
    AzureFoundry(super::azure_foundry::Provider),
    /// See [`bedrock::Provider`](super::bedrock::Provider).
    Bedrock(super::bedrock::Provider),
    /// See [`commandcode::Provider`](super::commandcode::Provider).
    Commandcode(super::commandcode::Provider),
    /// See [`commandcode_anthropic::Provider`](super::commandcode_anthropic::Provider).
    CommandcodeAnthropic(super::commandcode_anthropic::Provider),
    /// See [`copilot::Provider`](super::copilot::Provider).
    Copilot(super::copilot::Provider),
    /// See [`custom::Provider`](super::custom::Provider).
    Custom(super::custom::Provider),
    /// See [`deepinfra::Provider`](super::deepinfra::Provider).
    Deepinfra(super::deepinfra::Provider),
    /// See [`deepseek::Provider`](super::deepseek::Provider).
    Deepseek(super::deepseek::Provider),
    /// See [`fireworks::Provider`](super::fireworks::Provider).
    Fireworks(super::fireworks::Provider),
    /// See [`gemini::Provider`](super::gemini::Provider).
    Gemini(super::gemini::Provider),
    /// See [`gmi::Provider`](super::gmi::Provider).
    Gmi(super::gmi::Provider),
    /// See [`huggingface::Provider`](super::huggingface::Provider).
    Huggingface(super::huggingface::Provider),
    /// See [`kilocode::Provider`](super::kilocode::Provider).
    Kilocode(super::kilocode::Provider),
    /// See [`kimi_coding::Provider`](super::kimi_coding::Provider).
    KimiCoding(super::kimi_coding::Provider),
    /// See [`kimi_coding_cn::Provider`](super::kimi_coding_cn::Provider).
    KimiCodingCn(super::kimi_coding_cn::Provider),
    /// See [`meta_ai::Provider`](super::meta_ai::Provider).
    MetaAi(super::meta_ai::Provider),
    /// See [`minimax::Provider`](super::minimax::Provider).
    Minimax(super::minimax::Provider),
    /// See [`minimax_cn::Provider`](super::minimax_cn::Provider).
    MinimaxCn(super::minimax_cn::Provider),
    /// See [`minimax_oauth::Provider`](super::minimax_oauth::Provider).
    MinimaxOauth(super::minimax_oauth::Provider),
    /// See [`nebius_token_factory::Provider`](super::nebius_token_factory::Provider).
    NebiusTokenFactory(super::nebius_token_factory::Provider),
    /// See [`nous::Provider`](super::nous::Provider).
    Nous(super::nous::Provider),
    /// See [`novita::Provider`](super::novita::Provider).
    Novita(super::novita::Provider),
    /// See [`nvidia::Provider`](super::nvidia::Provider).
    Nvidia(super::nvidia::Provider),
    /// See [`ollama_cloud::Provider`](super::ollama_cloud::Provider).
    OllamaCloud(super::ollama_cloud::Provider),
    /// See [`openai_codex::Provider`](super::openai_codex::Provider).
    OpenaiCodex(super::openai_codex::Provider),
    /// See [`opencode_free::Provider`](super::opencode_free::Provider).
    OpencodeFree(super::opencode_free::Provider),
    /// See [`opencode_go::Provider`](super::opencode_go::Provider).
    OpencodeGo(super::opencode_go::Provider),
    /// See [`opencode_zen::Provider`](super::opencode_zen::Provider).
    OpencodeZen(super::opencode_zen::Provider),
    /// See [`openrouter::Provider`](super::openrouter::Provider).
    Openrouter(super::openrouter::Provider),
    /// See [`qwen_oauth::Provider`](super::qwen_oauth::Provider).
    QwenOauth(super::qwen_oauth::Provider),
    /// See [`router::Provider`](super::router::Provider).
    Router(super::router::Provider),
    /// See [`stepfun::Provider`](super::stepfun::Provider).
    Stepfun(super::stepfun::Provider),
    /// See [`upstage::Provider`](super::upstage::Provider).
    Upstage(super::upstage::Provider),
    /// See [`vertex::Provider`](super::vertex::Provider).
    Vertex(super::vertex::Provider),
    /// See [`xai::Provider`](super::xai::Provider).
    Xai(super::xai::Provider),
    /// See [`xiaomi::Provider`](super::xiaomi::Provider).
    Xiaomi(super::xiaomi::Provider),
    /// See [`zai::Provider`](super::zai::Provider).
    Zai(super::zai::Provider),
}
