//! The rule of the hold: who may hold a volume beside whom, decided
//! once, for every provider.
//!
//! A provider keeps the [`Holders`] of each volume under a lock of
//! its own and never decides what they allow: it hands them to
//! [`admit`], [`release`], [`lock`] and [`unlock`] and stores what
//! comes back. The rule is written here, in one place, and the
//! handlers read every refusal in the vocabulary of the wire.

use diverge_sdk::provider::endpoints::volumes::Mode;

use super::holders::Holders;

/// One of the two parties that take the SHARED hold on a volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Holder {
    /// A running container that mounts the volume, for the run's
    /// life; or an image build reading the volume as its context,
    /// for the build's.
    Run,
    /// A `volumes::fuse_serve` scope, for its life.
    FuseServe,
}

/// What the provider's capability for the volume's mode lets a holder
/// do — the handler's reading of the mode's `VolumeCapabilities` for
/// the holder's kind: `many` is `mount_many` for a run and
/// `fuse_serve_many` for a FUSE serve; `beside` is
/// `mount_and_fuse_serve` for either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Allow {
    /// Another holder of the SAME kind may already have the volume.
    pub many: bool,
    /// A holder of the OTHER kind may already have the volume.
    pub beside: bool,
}

/// Who has the volume, when a hold was not taken for that reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Held {
    /// A running container, or a build, has the volume, and the
    /// asker may not join it.
    Mounted,
    /// A FUSE serve has the volume, and the asker may not join it.
    FuseServed,
    /// An operation — a stat, a read, a write, a filetree, an edit
    /// or a delete — has the volume to itself.
    Locked,
}

/// Why a shared hold was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refused {
    /// The asker's `admit` declined the volume's mode, which this
    /// carries, and nothing was looked at: the mode is not the one
    /// the request meant, or the capability for it allows no holder
    /// of the asker's kind at all. The asker tells the two apart.
    Declined(Mode),
    /// The volume is held by a party the asker may not join.
    Held(Held),
}

/// The shared hold decided: `holders` as they are, `holder` asking
/// under `allow`; `Ok` is what the holders become, `Err` who refused.
///
/// The rules, applied in this order:
///
/// 1. A locked volume refuses every holder: [`Held::Locked`].
/// 2. A volume that holders of the SAME kind have refuses a holder
///    whose `many` is `false`: [`Held::Mounted`] for a run,
///    [`Held::FuseServed`] for a FUSE serve.
/// 3. A volume that holders of the OTHER kind have refuses a holder
///    whose `beside` is `false`: [`Held::FuseServed`] for a run,
///    [`Held::Mounted`] for a FUSE serve.
/// 4. Otherwise the holder is one more of its kind.
pub fn admit(holders: Holders, holder: Holder, allow: Allow) -> Result<Holders, Held> {
    let (runs, fuse_serves) = match holders {
        Holders::Locked => return Err(Held::Locked),
        Holders::Free => (0, 0),
        Holders::Shared { runs, fuse_serves } => (runs, fuse_serves),
    };
    match holder {
        Holder::Run => {
            if runs > 0 && !allow.many {
                return Err(Held::Mounted);
            }
            if fuse_serves > 0 && !allow.beside {
                return Err(Held::FuseServed);
            }
            Ok(Holders::Shared {
                runs: runs + 1,
                fuse_serves,
            })
        }
        Holder::FuseServe => {
            if fuse_serves > 0 && !allow.many {
                return Err(Held::FuseServed);
            }
            if runs > 0 && !allow.beside {
                return Err(Held::Mounted);
            }
            Ok(Holders::Shared {
                runs,
                fuse_serves: fuse_serves + 1,
            })
        }
    }
}

/// One holder of `holder`'s kind given back: what the holders become,
/// or `None` for a volume no such holder has, which changes nothing.
pub fn release(holders: Holders, holder: Holder) -> Option<Holders> {
    let Holders::Shared { runs, fuse_serves } = holders else {
        return None;
    };
    let (runs, fuse_serves) = match holder {
        Holder::Run => (runs.checked_sub(1)?, fuse_serves),
        Holder::FuseServe => (runs, fuse_serves.checked_sub(1)?),
    };
    Some(if runs == 0 && fuse_serves == 0 {
        Holders::Free
    } else {
        Holders::Shared { runs, fuse_serves }
    })
}

/// The exclusive hold decided: a free volume becomes
/// [`Locked`](Holders::Locked); a held one refuses — [`Held::Locked`]
/// for an operation, else [`Held::Mounted`] where any running
/// container has it, else [`Held::FuseServed`].
pub fn lock(holders: Holders) -> Result<Holders, Held> {
    match holders {
        Holders::Free => Ok(Holders::Locked),
        Holders::Locked => Err(Held::Locked),
        Holders::Shared { runs, .. } if runs > 0 => Err(Held::Mounted),
        Holders::Shared { .. } => Err(Held::FuseServed),
    }
}

/// The exclusive hold given back: [`Free`](Holders::Free), or `None`
/// for a volume that was not locked, which changes nothing.
pub fn unlock(holders: Holders) -> Option<Holders> {
    match holders {
        Holders::Locked => Some(Holders::Free),
        Holders::Free | Holders::Shared { .. } => None,
    }
}
