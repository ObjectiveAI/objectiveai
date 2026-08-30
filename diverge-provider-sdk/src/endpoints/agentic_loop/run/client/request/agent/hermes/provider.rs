//! The inference sources Hermes speaks to.

use serde::{Deserialize, Serialize};

/// One of Hermes's bundled provider profiles, plus
/// [`auto`](Self::Auto) — the registry shipped at the pinned Hermes
/// version (`plugins/model-providers/`), which is the vocabulary its
/// configuration accepts.
///
/// CLOSED at the pin, deliberately: user plugins can extend Hermes's
/// registry, but the container runs stock Hermes, so the bundled set
/// is the whole truth. Providers whose auth is interactive by nature
/// (the OAuth tiers — `nous`, `openai-codex`, `qwen-oauth`,
/// `copilot`, …) are still named: credentials are the caller's to
/// provision (the request's environment, or a mount), and picking a
/// provider without workable ones fails as Hermes's own error, not
/// as protocol.
///
/// Local runtimes — ollama, vllm, llama.cpp — are
/// [`custom`](Self::Custom) plus a
/// [`base_url`](super::Agent::base_url), exactly as Hermes itself
/// aliases them.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    /// Hermes detects the provider from the credentials it finds, in
    /// its own fixed priority: an OpenRouter/OpenAI key means
    /// openrouter; else the first provider-specific key present
    /// (GLM, Kimi, MiniMax, …) means that provider; else a
    /// logged-in OAuth session; else AWS credentials mean bedrock;
    /// else the run fails as unconfigured. In this protocol that
    /// means THE REQUEST'S ENVIRONMENT DECIDES — ship
    /// `OPENROUTER_API_KEY` and auto is openrouter, ship only
    /// `GLM_API_KEY` and auto is zai.
    #[default]
    Auto,
    /// Actual Computer.
    Actual,
    /// Vercel AI Gateway.
    AiGateway,
    /// Alibaba Cloud (DashScope).
    Alibaba,
    /// Alibaba Cloud's dedicated coding tier.
    AlibabaCodingPlan,
    /// Anthropic, over the Messages API.
    Anthropic,
    /// Arcee AI.
    Arcee,
    /// Microsoft Azure Foundry (user-supplied endpoint).
    AzureFoundry,
    /// AWS Bedrock (SDK credentials, not an API key).
    Bedrock,
    /// CommandCode.
    Commandcode,
    /// CommandCode's Anthropic-dialect route.
    CommandcodeAnthropic,
    /// GitHub Copilot (token exchange).
    Copilot,
    /// GitHub Copilot as an ACP subprocess.
    CopilotAcp,
    /// Any OpenAI-compatible endpoint the caller names — the local
    /// runtimes' home.
    Custom,
    /// DeepInfra.
    Deepinfra,
    /// DeepSeek.
    Deepseek,
    /// Fireworks AI.
    Fireworks,
    /// Google Gemini (AI Studio).
    Gemini,
    /// GMI Cloud.
    Gmi,
    /// HuggingFace's inference router.
    Huggingface,
    /// Kilo Code's gateway.
    Kilocode,
    /// Moonshot's Kimi coding tier.
    KimiCoding,
    /// Moonshot's Kimi coding tier, China endpoint.
    KimiCodingCn,
    /// Meta's Model API (Muse Spark).
    MetaAi,
    /// MiniMax.
    Minimax,
    /// MiniMax, China endpoint.
    MinimaxCn,
    /// MiniMax over OAuth.
    MinimaxOauth,
    /// Nebius Token Factory.
    NebiusTokenFactory,
    /// Nous Research — the Portal, Hermes's home team.
    Nous,
    /// NovitaAI.
    Novita,
    /// NVIDIA NIM.
    Nvidia,
    /// Ollama Cloud (the hosted service, not a local ollama — that
    /// is [`custom`](Self::Custom)).
    OllamaCloud,
    /// The ChatGPT Codex backend, over external OAuth.
    OpenaiCodex,
    /// OpenCode's free tier — keyless.
    OpencodeFree,
    /// OpenCode Go.
    OpencodeGo,
    /// OpenCode Zen.
    OpencodeZen,
    /// OpenRouter.
    Openrouter,
    /// Qwen's portal, over OAuth.
    QwenOauth,
    /// Ramp Router.
    Router,
    /// StepFun's step plan.
    Stepfun,
    /// Upstage Solar.
    Upstage,
    /// GCP Vertex AI (service account / ADC, not an API key).
    Vertex,
    /// xAI.
    Xai,
    /// Xiaomi MiMo.
    Xiaomi,
    /// Z.AI (GLM / Zhipu).
    Zai,
}
