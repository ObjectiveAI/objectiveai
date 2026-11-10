//! Provider definitions — the inference source, and the secrets it
//! takes.
//!
//! AUTH IS A SECRET. Each provider here is a structure carrying its
//! discriminator and the endpoint facts that provider needs — never
//! a credential. A credential is a SECRET: the agent's template
//! declares it by NAME, the program answers its registration with
//! the names its arguments select and the mode it takes each in
//! (every structure's `SECRET:` paragraph says which), and the proxy
//! feeds the values on `POST /secrets` before every loop. Nothing
//! about a run's inference auth rides the arguments, the filesystem
//! or a mount — tool credentials (`FAL_KEY`, `HASS_TOKEN`, and
//! friends) are secrets on the toolset structures the same way.
//!
//! HOW THE HARNESS APPLIES IT: each structure's doc names the
//! mechanism, and all but two reduce to setting the provider's
//! canonical env var(s) — the secret's own name — in the gateway's
//! PROCESS environment before Hermes starts, to the fed value. The
//! two: [`custom`]'s key goes into `model.api_key` of the config,
//! the only place Hermes reads a custom key from; [`vertex`]'s
//! document goes to the file `VERTEX_CREDENTIALS_PATH` names.
//! Process env is load-bearing twice over: Hermes prefers
//! `~/.hermes/.env` OVER process env when that file exists — so the
//! image must guarantee it does not — and bedrock's botocore chain
//! reads only real env, never Hermes's dotenv.
//!
//! ROTATING OAUTH IS A ROTATING SECRET. Four providers — [`nous`],
//! [`openai_codex`], [`minimax_oauth`], [`qwen_oauth`] —
//! authenticate with OAuth state whose refresh tokens are
//! SINGLE-USE: every refresh rotates them, and whoever holds the
//! old copy is logged out. A value read and forgotten cannot carry
//! that, so these structures declare their state document as a
//! `rotating` secret — a JSON object the harness writes where Hermes
//! reads it before the gateway starts, reads back as the run left it
//! after the gateway exits, and hands to the proxy's collect, which
//! writes it back where it was locked from. The lock the caller
//! holds on a rotating value for the container's active period is
//! what keeps two runs from rotating one login past each other.
//! (These providers' advertised key env vars — `NOUS_API_KEY`,
//! `QWEN_API_KEY` — are vestigial; no resolution path reads them, so
//! the state document is the only credential there is.)
//!
//! DROPPED from Hermes's bundled registry: `copilot-acp` alone,
//! which keeps its auth inside a spawned external CLI, not in
//! Hermes at all. Keyed Copilot remains as [`copilot`].

pub mod actual;
pub mod ai_gateway;
pub mod alibaba;
pub mod alibaba_coding_plan;
pub mod anthropic;
pub mod arcee;
pub mod azure_foundry;
pub mod bedrock;
pub mod commandcode;
pub mod commandcode_anthropic;
pub mod copilot;
pub mod custom;
pub mod deepinfra;
pub mod deepseek;
pub mod fireworks;
pub mod gemini;
pub mod gmi;
pub mod huggingface;
pub mod kilocode;
pub mod kimi_coding;
pub mod kimi_coding_cn;
pub mod meta_ai;
pub mod minimax;
pub mod minimax_cn;
pub mod minimax_oauth;
pub mod nebius_token_factory;
pub mod nous;
pub mod novita;
pub mod nvidia;
pub mod ollama_cloud;
pub mod openai_codex;
pub mod opencode_free;
pub mod opencode_go;
pub mod opencode_zen;
pub mod openrouter;
pub mod qwen_oauth;
pub mod router;
pub mod stepfun;
pub mod upstage;
pub mod vertex;
pub mod xai;
pub mod xiaomi;
pub mod zai;

mod provider;

pub use provider::*;
