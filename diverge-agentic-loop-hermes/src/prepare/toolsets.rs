//! The toolsets' contribution: which are exposed, their credentials
//! as environment, their backend pins, spotify's state as a resource.

use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::hermes::Toolsets;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::hermes::toolsets::browser::Remote;
use diverge_provider_sdk::endpoints::agentic_loop::run::client::request::agent::hermes::toolsets::web::{Extract, Search};

use super::{Ask, Plan, PrepareError, Target};

/// Add the toolsets to the plan.
///
/// # Membership
///
/// Hermes exposes, on its API server, exactly the toolsets listed
/// under `platform_toolsets.api_server` — by its own configurable-key
/// names, in its own order — and an explicit list is the only
/// deterministic form. So one is always written, and each toolset's
/// place in it is decided the same way: present in the request
/// (`true`, or a structure) → in; `false` → out; unsaid → what
/// Hermes's own API-server default would do, which is ON for web,
/// browser, terminal, file, code_execution, vision, todo, memory and
/// session_search, and OFF for video, video_gen, x_search, tts,
/// homeassistant and spotify — and for image_gen, which Hermes lists
/// by default but hides without a FAL key, and only the structure
/// brings one. `skills` is always in:
/// the mounts decide whether there are any. Never in: delegation
/// and cronjob (both in Hermes's default, both unsupportable here),
/// clarify, computer_use, discord, yuanbao, context_engine, stt.
///
/// # Arguments
///
/// Each structure's credentials become the variables the SDK's
/// toolset docs name; a web slot also pins its backend; an
/// ElevenLabs key pins `tts.provider`; the generation toolsets pin
/// `fal`; spotify's state is a resource bound for `auth.json`.
pub fn apply(toolsets: &Toolsets, plan: &mut Plan) -> Result<(), PrepareError> {
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
    switch(&mut on, "web", toolsets.web.is_some(), None, true);

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
            }
            None => {}
        }
    }
    switch(&mut on, "browser", toolsets.browser.is_some(), None, true);

    switch(&mut on, "terminal", false, toolsets.terminal, true);
    switch(&mut on, "file", false, toolsets.file, true);
    switch(&mut on, "code_execution", false, toolsets.code_execution, true);
    switch(&mut on, "vision", false, toolsets.vision, true);
    switch(&mut on, "video", false, toolsets.video, false);

    if let Some(image_gen) = &toolsets.image_gen {
        plan.set("FAL_KEY", image_gen.fal_key.clone())?;
        plan.image_gen = true;
    }
    switch(&mut on, "image_gen", toolsets.image_gen.is_some(), None, false);

    if let Some(video_gen) = &toolsets.video_gen {
        plan.set("FAL_KEY", video_gen.fal_key.clone())?;
        plan.video_gen = true;
    }
    switch(&mut on, "video_gen", toolsets.video_gen.is_some(), None, false);

    if let Some(x_search) = &toolsets.x_search {
        plan.set("XAI_API_KEY", x_search.api_key.clone())?;
    }
    switch(&mut on, "x_search", toolsets.x_search.is_some(), None, false);

    if let Some(tts) = &toolsets.tts
        && let Some(elevenlabs_api_key) = &tts.elevenlabs_api_key
    {
        plan.set("ELEVENLABS_API_KEY", elevenlabs_api_key.clone())?;
        plan.tts_elevenlabs = true;
    }
    switch(&mut on, "tts", toolsets.tts.is_some(), None, false);

    on.push("skills");
    switch(&mut on, "todo", false, toolsets.todo, true);
    switch(&mut on, "memory", false, toolsets.memory, true);
    switch(&mut on, "session_search", false, toolsets.session_search, true);

    if let Some(homeassistant) = &toolsets.homeassistant {
        plan.set("HASS_URL", homeassistant.url.clone())?;
        plan.set("HASS_TOKEN", homeassistant.token.clone())?;
    }
    switch(
        &mut on,
        "homeassistant",
        toolsets.homeassistant.is_some(),
        None,
        false,
    );

    if let Some(spotify) = &toolsets.spotify {
        plan.set("HERMES_SPOTIFY_CLIENT_ID", spotify.client_id.clone())?;
        plan.asks.push(Ask {
            field: "toolsets.spotify.auth_resource",
            identity: spotify.auth_resource.clone(),
            target: Target::AuthEntry("spotify"),
        });
    }
    switch(&mut on, "spotify", toolsets.spotify.is_some(), None, false);

    plan.toolsets = on;
    Ok(())
}

/// Decide one toolset's place in the list: a present structure is
/// in; a boolean says so itself; unsaid takes Hermes's default.
fn switch(
    on: &mut Vec<&'static str>,
    name: &'static str,
    present: bool,
    said: Option<bool>,
    default: bool,
) {
    if present || said.unwrap_or(default) {
        on.push(name);
    }
}
