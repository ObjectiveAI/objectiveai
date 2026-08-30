//! Provider definitions — the inference source and its credentials.
//!
//! AUTH IS AN ARGUMENT. Each provider here is a structure carrying
//! exactly the credentials and endpoint facts that provider needs;
//! nothing about a run's inference auth rides the filesystem, a
//! mount, or the request's `environment` (which remains the TOOL
//! credential channel — `FAL_KEY`, `HASS_TOKEN`, and friends).
//!
//! HOW THE HARNESS APPLIES IT: each structure's doc names the
//! mechanism, and all but one reduce to setting the provider's
//! canonical env var(s) in the gateway's PROCESS environment before
//! Hermes starts (vertex additionally writes its service-account
//! document to a file). Process env is load-bearing twice over:
//! Hermes prefers `~/.hermes/.env` OVER process env when that file
//! exists — so the image must guarantee it does not — and bedrock's
//! botocore chain reads only real env, never Hermes's dotenv.
//!
//! DROPPED from Hermes's bundled registry, because a credential
//! that cannot be handed over as a value is not an argument:
//! `nous`, `openai-codex`, `minimax-oauth` and `qwen-oauth` are
//! OAuth with single-use rotating refresh tokens — a seeded token
//! would be burned by the container's first refresh, killing the
//! caller's own login (their advertised key env vars are vestigial;
//! no resolution path reads them) — and `copilot-acp` keeps its
//! auth inside a spawned external CLI, not in Hermes at all. Keyed
//! MiniMax and Copilot remain as [`minimax`]/[`minimax_cn`] and
//! [`copilot`].

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
pub mod nebius_token_factory;
pub mod novita;
pub mod nvidia;
pub mod ollama_cloud;
pub mod opencode_free;
pub mod opencode_go;
pub mod opencode_zen;
pub mod openrouter;
pub mod router;
pub mod stepfun;
pub mod upstage;
pub mod vertex;
pub mod xai;
pub mod xiaomi;
pub mod zai;

mod provider;

pub use provider::*;
