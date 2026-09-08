//! The toolsets' contribution: which are exposed, their credentials
//! as environment, their backend pins, spotify's login as a vault
//! document.

use diverge_provider_sdk::shared::containers::vault::keys;

use super::{Document, Plan, PrepareError, Target};
use crate::agent::Toolsets;
use crate::agent::toolsets::browser::Remote;
use crate::agent::toolsets::web::{Extract, Search};

/// Add the toolsets to the plan.
///
/// # Membership
///
/// Hermes exposes, on its API server, exactly the toolsets listed
/// under `platform_toolsets.api_server` — by its own configurable-key
/// names, in its own order — and an explicit list is the only
/// deterministic form. So one is always written, and each toolset's
/// place in it is decided the same way: `true`, or a structure
/// present, → in; `false`, absent, or a structure absent, → out.
/// Nothing is on by omission — Hermes's own
/// API-server default (which would list web, browser, terminal,
/// file, code_execution, vision, todo, memory, session_search and
/// image_gen) is never consulted. `skills` is in exactly when
/// something is mounted under the external skills path — `skills`
/// here is that fact, decided by the caller of this function from
/// the disk — and out otherwise: a skills catalogue with nothing of the
/// caller's in it is only Hermes's bundle, which nobody asked for.
/// Never in: delegation
/// and cronjob (both in Hermes's default, both unsupportable here),
/// clarify, computer_use, discord, yuanbao, context_engine, stt.
///
/// # Arguments
///
/// Each structure's credentials become the variables the SDK's
/// toolset docs name; a web slot also pins its backend; an
/// ElevenLabs key pins `tts.provider`; the generation toolsets pin
/// `fal`; spotify's login is a vault document bound for `auth.json`.
pub fn apply(
    toolsets: &Toolsets,
    skills: bool,
    plan: &mut Plan,
) -> Result<(), PrepareError> {
    let mut on: Vec<&'static str> = Vec::new();

    if let Some(web) = &toolsets.web {
        if let Some(search) = &web.search {
            let backend = match search {
                Search::Tavily { tavily_api_key } => {
                    plan.set("TAVILY_API_KEY", tavily_api_key.clone())?;
                    "tavily"
                }
                Search::Exa { exa_api_key } => {
                    plan.set("EXA_API_KEY", exa_api_key.clone())?;
                    "exa"
                }
                Search::Parallel { parallel_api_key } => {
                    plan.set("PARALLEL_API_KEY", parallel_api_key.clone())?;
                    "parallel"
                }
                Search::Keenable { keenable_api_key } => {
                    plan.set("KEENABLE_API_KEY", keenable_api_key.clone())?;
                    "keenable"
                }
                Search::Brave { brave_search_api_key } => {
                    plan.set(
                        "BRAVE_SEARCH_API_KEY",
                        brave_search_api_key.clone(),
                    )?;
                    "brave-free"
                }
                Search::Searxng { searxng_url } => {
                    plan.set("SEARXNG_URL", searxng_url.clone())?;
                    "searxng"
                }
            };
            plan.search_backend = Some(backend);
        }
        if let Some(Extract::Firecrawl { firecrawl_api_key }) = &web.extract {
            plan.set("FIRECRAWL_API_KEY", firecrawl_api_key.clone())?;
            plan.extract_backend = Some("firecrawl");
        }
    }
    switch(&mut on, "web", toolsets.web.is_some());

    if let Some(browser) = &toolsets.browser {
        match &browser.remote {
            Some(Remote::Cdp { cdp_url }) => {
                plan.set("BROWSER_CDP_URL", cdp_url.clone())?;
            }
            Some(Remote::Browserbase {
                browserbase_api_key,
                browserbase_project_id,
            }) => {
                plan.set("BROWSERBASE_API_KEY", browserbase_api_key.clone())?;
                plan.set(
                    "BROWSERBASE_PROJECT_ID",
                    browserbase_project_id.clone(),
                )?;
                plan.browserbase = true;
            }
            None => {}
        }
    }
    switch(&mut on, "browser", toolsets.browser.is_some());

    switch(&mut on, "terminal", toolsets.terminal.unwrap_or(false));
    switch(&mut on, "file", toolsets.file.unwrap_or(false));
    switch(&mut on, "code_execution", toolsets.code_execution.unwrap_or(false));
    switch(&mut on, "vision", toolsets.vision.unwrap_or(false));
    switch(&mut on, "video", toolsets.video.unwrap_or(false));

    if let Some(image_gen) = &toolsets.image_gen {
        plan.set("FAL_KEY", image_gen.fal_key.clone())?;
        plan.image_gen = true;
    }
    switch(&mut on, "image_gen", toolsets.image_gen.is_some());

    if let Some(video_gen) = &toolsets.video_gen {
        plan.set("FAL_KEY", video_gen.fal_key.clone())?;
        plan.video_gen = true;
    }
    switch(&mut on, "video_gen", toolsets.video_gen.is_some());

    if let Some(x_search) = &toolsets.x_search {
        plan.set("XAI_API_KEY", x_search.api_key.clone())?;
    }
    switch(&mut on, "x_search", toolsets.x_search.is_some());

    if let Some(tts) = &toolsets.tts
        && let Some(elevenlabs_api_key) = &tts.elevenlabs_api_key
    {
        plan.set("ELEVENLABS_API_KEY", elevenlabs_api_key.clone())?;
        plan.tts_elevenlabs = true;
    }
    switch(&mut on, "tts", toolsets.tts.is_some());

    if skills {
        on.push("skills");
    }
    switch(&mut on, "todo", toolsets.todo.unwrap_or(false));
    switch(&mut on, "memory", toolsets.memory.unwrap_or(false));
    switch(&mut on, "session_search", toolsets.session_search.unwrap_or(false));

    if let Some(homeassistant) = &toolsets.homeassistant {
        plan.set("HASS_URL", homeassistant.url.clone())?;
        plan.set("HASS_TOKEN", homeassistant.token.clone())?;
    }
    switch(&mut on, "homeassistant", toolsets.homeassistant.is_some());

    if let Some(spotify) = &toolsets.spotify {
        plan.set("HERMES_SPOTIFY_CLIENT_ID", spotify.client_id.clone())?;
        plan.documents.push(Document {
            key: keys::SPOTIFY_OAUTH,
            target: Target::AuthEntry("spotify"),
        });
    }
    switch(&mut on, "spotify", toolsets.spotify.is_some());

    plan.toolsets = on;
    Ok(())
}

/// Decide one toolset's place in the list: in when its switch is
/// thrown — a `true`, or a structure present.
fn switch(on: &mut Vec<&'static str>, name: &'static str, thrown: bool) {
    if thrown {
        on.push(name);
    }
}
