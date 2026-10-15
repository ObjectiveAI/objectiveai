//! Whether a provider's capabilities hold out what a run, or a
//! mount, needs of it.
//!
//! The daemon asks a provider nothing its cached `info` excludes:
//! every choice of a provider for a container, and every serve a
//! mount needs, is judged here first, against the provider's
//! capabilities as it answered them, and a provider that lacks one is
//! passed over — or, pinned, refused — without being asked.

use diverge_sdk::provider::endpoints::info::server::response::Frame as Info;
use diverge_sdk::provider::endpoints::volumes::Mode;

/// What a container's run asks of the provider it runs on, beyond
/// running a container: FUSE mounts the daemon serves into it, and
/// the provider's own volumes mounted into it, by mode.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Needs {
    /// The run names at least one FUSE mount — a cross-provider
    /// volume, or a dependency's mount of the agent's path — which
    /// the provider must make.
    pub fuse_mounts: bool,
    /// The modes of the provider's own volumes the run mounts, each
    /// of which the provider must mount.
    pub volume_modes: Vec<Mode>,
}

/// `Ok` when `info` holds out everything `needs`; else what it lacks,
/// in a sentence a start error carries.
pub fn check(info: &Info, needs: &Needs) -> Result<(), String> {
    if needs.fuse_mounts && !info.capabilities.containers_fuse_mount {
        return Err("the provider makes no FUSE mount, which the container's mounts need".to_string());
    }
    for mode in &needs.volume_modes {
        match info.capabilities.volume(*mode) {
            Some(allowed) if allowed.mount => {}
            _ => return Err(format!("the provider mounts no volume whose mode is {}", name(*mode))),
        }
    }
    Ok(())
}

/// `Ok` when `info` holds out a FUSE serve of a volume whose mode is
/// `mode`; else what it lacks, in a sentence.
pub fn fuse_serve(info: &Info, mode: Mode) -> Result<(), String> {
    match info.capabilities.volume(mode) {
        Some(allowed) if allowed.fuse_serve => Ok(()),
        _ => Err(format!("the provider FUSE serves no volume whose mode is {}", name(mode))),
    }
}

/// A mode's name, as the wire spells it.
pub fn name(mode: Mode) -> &'static str {
    match mode {
        Mode::Persistent => "persistent",
        Mode::Ephemeral => "ephemeral",
        Mode::ReadOnly => "read_only",
    }
}
