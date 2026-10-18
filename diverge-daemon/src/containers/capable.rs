//! Whether a provider's capabilities hold out what a run, or a
//! mount, needs of it — and whether it can source the run's image.
//!
//! The daemon asks a provider nothing its cached `info` excludes:
//! every choice of a provider for a container, and every volume
//! serve a mount needs, is judged here first, against the provider's
//! capabilities as it answered them, and a provider that lacks one is
//! passed over — or, pinned, refused — without being asked. The
//! image is judged the same way, from the capabilities first — see
//! [`registry`] — and from who holds it only when no registry will
//! do. And among the providers that pass, the one with the most room
//! for the run by its last capacity answer is tried first: see
//! [`headroom`].

use diverge_sdk::provider::endpoints::capacity::server::response::Frame as Capacity;
use diverge_sdk::provider::endpoints::info::server::response::Frame as Info;
use diverge_sdk::provider::endpoints::volumes::Mode;
use diverge_sdk::shared::containers::request::Image;

/// What a container's run asks of the provider it runs on, beyond
/// running a container: the provider's own volumes mounted into it,
/// by mode, and the image the container is made from, which the
/// provider must be able to obtain. Every provider makes the FUSE
/// mounts a run names and FUSE serves a running container, so a run
/// asks nothing of either.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Needs {
    /// The modes of the provider's own volumes the run mounts, each
    /// of which the provider must mount.
    pub volume_modes: Vec<Mode>,
    /// The image the container is made from, its digest and its
    /// references, which the provider must be able to source: see
    /// [`registry`]. `None` asks nothing of the image.
    pub image: Option<Image>,
    /// The memory the run reserves, in bytes: what [`headroom`]
    /// measures the provider's `container_memory` against. `0` asks
    /// nothing of it.
    pub memory: u64,
    /// The overlay disk the run reserves, in bytes: what
    /// [`headroom`] measures the provider's `container_disk` against.
    /// `0` asks nothing of it.
    pub disk: u64,
}

/// What a headroom ratio is scaled by: one is a run's worth of room
/// exactly, in millionths, so that a ratio compares as an integer.
const SCALE: u128 = 1_000_000;

/// How many runs' worth of room the provider's last capacity holds
/// for a run of these needs, in millionths: the smaller of its
/// `container_memory` over the run's memory and its `container_disk`
/// over the run's disk — the scarcer of the two is what the run
/// comes up against — with a need of `0` counting as unbounded. The
/// chooser tries the greatest first: see
/// [`candidates`](super::provider::candidates). A ratio under one
/// is a provider short of room by its last answer; it is sorted after
/// every provider with room, and still tried, since the answer may
/// be stale.
pub fn headroom(capacity: &Capacity, needs: &Needs) -> u128 {
    let memory = ratio(capacity.container_memory, needs.memory);
    let disk = ratio(capacity.container_disk, needs.disk);
    memory.min(disk)
}

/// `room` over `need`, in millionths; `u128::MAX` for a need of `0`.
fn ratio(room: u64, need: u64) -> u128 {
    if need == 0 {
        return u128::MAX;
    }
    u128::from(room).saturating_mul(SCALE) / u128::from(need)
}

/// `Ok` when `info` holds out everything `needs`; else what it lacks,
/// in a sentence a start error carries.
pub fn check(info: &Info, needs: &Needs) -> Result<(), String> {
    for mode in &needs.volume_modes {
        match info.capabilities.volume(*mode) {
            Some(allowed) if allowed.mount => {}
            _ => return Err(format!("the provider mounts no volume whose mode is {}", name(*mode))),
        }
    }
    Ok(())
}

/// `Ok` when the provider answering `info` lists a registry that a
/// reference of `image` names, from which it can pull the image for
/// a run; else why not, in a sentence.
///
/// # Where a run's image may come from
///
/// The protocol has a provider obtain a run's image from what it
/// holds, from the caller, or from a registry its capabilities list
/// that a reference of the image names — and from no other registry.
/// This is the registry test alone, answered from the cached info
/// and nothing else, so it costs no ask. Whether the image is held —
/// in some connected provider's client image store, this provider's
/// own or another's from which the daemon supplies it over the run's
/// OCI channels — is the chooser's to ask, and it asks only when no
/// candidate passes this test: see
/// [`candidates`](super::provider::candidates). What a provider
/// merely caches from an earlier run is counted nowhere, since no
/// check can ask a provider about its cache.
pub fn registry(info: &Info, image: &Image) -> Result<(), String> {
    if image.references.is_empty() {
        return Err("the image names no registry".to_string());
    }
    let listed = image
        .references
        .iter()
        .any(|reference| info.capabilities.images_registries.contains(&reference.registry));
    if listed {
        Ok(())
    } else {
        Err("the provider pulls from no registry the image's references name".to_string())
    }
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
