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
//! ROTATING OAUTH IS A RESOURCE. Four providers — [`nous`],
//! [`openai_codex`], [`minimax_oauth`], [`qwen_oauth`] —
//! authenticate with OAuth state whose refresh tokens are
//! SINGLE-USE: every refresh rotates them, and whoever holds the
//! old copy is logged out. A plain value argument cannot carry
//! that, so their credential fields are RESOURCES (named
//! `*_resource`): caller-provided state the run MUTATES, whose
//! rotated form is surfaced back to the caller so their next
//! request carries current credentials. A resource field carries
//! the state's size-bearing IDENTITY — the file grammar,
//! `f1:<size>:<base64url sha256 of the bytes>` — never the bytes:
//! those live with the client and arrive over the
//! [`FetchResource`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchResource)
//! exchange, the way mounted content does. How a rotated resource
//! travels back is the protocol's resource mechanism, not this
//! module's concern; what this module fixes is which documents are
//! resources and how the harness seeds them. (These providers'
//! advertised key env vars — `NOUS_API_KEY`, `QWEN_API_KEY` — are
//! vestigial; no resolution path reads them, so the state document
//! is the only credential there is.)
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
