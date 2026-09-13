//! Well-known keys: credentials more than one image may share.
//!
//! A vault key is one credential's home. An image that needs a
//! document another image also needs — an OAuth login, rotated by
//! whichever vendor CLI holds it — names it by the constant here,
//! so the caller keeps ONE copy and every image finds it under the
//! same name. Each value is the whole JSON document of that flow,
//! exactly as the vendor's own tooling keeps it, bytes verbatim.
//!
//! # The cycle an image owes a rotating document
//!
//! A rotating credential is mutated by the run — a single-use
//! refresh token is burned and replaced — so the copy in the vault
//! must be the current one when the next run reads it. An image
//! that uses one of these keys locks it first, reads it, writes it
//! where its tooling reads it, runs, reads the document back as the
//! run left it, sets it, and unlocks: lock, get, run, set, unlock.
//! The lock is held for the run — refreshed before its TTL runs out
//! — so two runs of two images never rotate the same login past
//! each other.

/// Nous Research's Portal login: the `providers.nous` entry of a
/// Hermes `auth.json`, with `access_token` and `refresh_token`.
pub const NOUS_OAUTH: &str = "NOUS_OAUTH";

/// The ChatGPT Codex backend's login: the `providers.openai-codex`
/// entry of a Hermes `auth.json`, its `tokens.access_token` and
/// `tokens.refresh_token` the working pair.
pub const OPENAI_CODEX_OAUTH: &str = "OPENAI_CODEX_OAUTH";

/// MiniMax's OAuth login: the `providers.minimax-oauth` entry of a
/// Hermes `auth.json`, complete — `access_token`, `refresh_token`,
/// `portal_base_url`, `inference_base_url`, `client_id`,
/// `expires_at`.
pub const MINIMAX_OAUTH: &str = "MINIMAX_OAUTH";

/// Qwen's portal login: the Qwen CLI's own `oauth_creds.json` —
/// `access_token`, `refresh_token`, `token_type`, `resource_url`,
/// `expiry_date` in epoch milliseconds.
pub const QWEN_OAUTH: &str = "QWEN_OAUTH";

/// A Spotify PKCE login: the `providers.spotify` entry of a Hermes
/// `auth.json`, `access_token` and `refresh_token` the working pair.
pub const SPOTIFY_OAUTH: &str = "SPOTIFY_OAUTH";
