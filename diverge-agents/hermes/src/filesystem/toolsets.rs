//! The toolsets' contribution: which are exposed, their credentials
//! as the secrets the plan declares, their endpoints as environment,
//! their backend pins.

use super::{Plan, PrepareError, Secret, Target};
use crate::agent::Toolsets;
use crate::agent::toolsets::browser::Remote;
use crate::agent::toolsets::web::{Extract, Keyed, Search};

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
/// Each structure's credentials are the secrets the agent's toolset
/// docs name, declared here bound for the environment; its endpoints
/// are plain variables; a web slot also pins its backend; ElevenLabs
/// pins `tts.provider`; the generation toolsets pin `fal`; spotify's
/// login is a rotating secret bound for `auth.json`.
pub fn apply(
    toolsets: &Toolsets,
    skills: bool,
    plan: &mut Plan,
) -> Result<(), PrepareError> {
    let mut on: Vec<&'static str> = Vec::new();

    if let Some(web) = &toolsets.web {
        if let Some(search) = &web.search {
            let backend = match search {
                Search::Keyed(keyed) => {
                    let (name, backend) = match keyed {
                        Keyed::Tavily => ("TAVILY_API_KEY", "tavily"),
                        Keyed::Exa => ("EXA_API_KEY", "exa"),
                        Keyed::Parallel => ("PARALLEL_API_KEY", "parallel"),
                        Keyed::Keenable => ("KEENABLE_API_KEY", "keenable"),
                        Keyed::BraveFree => ("BRAVE_SEARCH_API_KEY", "brave-free"),
                    };
                    env(plan, name)?;
                    backend
                }
                Search::Searxng { searxng_url } => {
                    plan.set("SEARXNG_URL", searxng_url.clone())?;
                    "searxng"
                }
            };
            plan.search_backend = Some(backend);
        }
        if let Some(Extract::Firecrawl) = &web.extract {
            env(plan, "FIRECRAWL_API_KEY")?;
            plan.extract_backend = Some("firecrawl");
        }
    }
    switch(&mut on, "web", toolsets.web.is_some());

    if let Some(browser) = &toolsets.browser {
        match &browser.remote {
            Some(Remote::Cdp { cdp_url }) => {
                plan.set("BROWSER_CDP_URL", cdp_url.clone())?;
            }
            Some(Remote::Browserbase(_)) => {
                env(plan, "BROWSERBASE_API_KEY")?;
                env(plan, "BROWSERBASE_PROJECT_ID")?;
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

    if toolsets.image_gen.unwrap_or(false) {
        env(plan, "FAL_KEY")?;
        plan.image_gen = true;
    }
    switch(&mut on, "image_gen", toolsets.image_gen.unwrap_or(false));

    if toolsets.video_gen.unwrap_or(false) {
        env(plan, "FAL_KEY")?;
        plan.video_gen = true;
    }
    switch(&mut on, "video_gen", toolsets.video_gen.unwrap_or(false));

    if toolsets.x_search.unwrap_or(false) {
        env(plan, "XAI_API_KEY")?;
    }
    switch(&mut on, "x_search", toolsets.x_search.unwrap_or(false));

    if let Some(tts) = &toolsets.tts {
        if tts.elevenlabs {
            env(plan, "ELEVENLABS_API_KEY")?;
            plan.tts_elevenlabs = true;
        }
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
        env(plan, "HASS_TOKEN")?;
    }
    switch(&mut on, "homeassistant", toolsets.homeassistant.is_some());

    if let Some(spotify) = &toolsets.spotify {
        plan.set("HERMES_SPOTIFY_CLIENT_ID", spotify.client_id.clone())?;
        plan.declare(Secret {
            name: "SPOTIFY_OAUTH",
            target: Target::AuthEntry("spotify"),
        })?;
    }
    switch(&mut on, "spotify", toolsets.spotify.is_some());

    plan.toolsets = on;
    Ok(())
}

/// Declare one secret bound for the gateway's environment, under
/// its own name.
fn env(plan: &mut Plan, name: &'static str) -> Result<(), PrepareError> {
    plan.declare(Secret {
        name,
        target: Target::Env,
    })
}

/// Decide one toolset's place in the list: in when its switch is
/// thrown — a `true`, or a structure present.
fn switch(on: &mut Vec<&'static str>, name: &'static str, thrown: bool) {
    if thrown {
        on.push(name);
    }
}
