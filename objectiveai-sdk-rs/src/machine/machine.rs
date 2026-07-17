//! Machine identity: a stable, privacy-preserving identifier for the
//! machine a process runs on, plus display metadata (OS + hostname).
//!
//! The identity `id` is `hex(sha256("objectiveai-machine-id" || raw))`
//! where `raw` is the platform's installation identifier — Linux
//! `/etc/machine-id`, Windows `MachineGuid`, macOS `IOPlatformUUID` —
//! hashed so the OS-global identifier is never sent raw (the salt also
//! decorrelates it from other software's use of the same source). Two
//! processes on one machine compute the SAME id independently, with no
//! shared state — that is the property the laboratories host protocol
//! relies on for local-vs-remote classification. When no OS identifier
//! is readable, a UUID persisted at `<objectiveai_dir>/bin/machine-id`
//! is hashed instead (stable per install dir).
//!
//! `os` is `std::env::consts::OS` verbatim ("windows" / "linux" /
//! "macos" / …) — a display discriminator (per-OS icons); consumers
//! must render unknown values generically. `hostname` is a display
//! label only, never identity.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The machine a process runs on: stable hashed `id` (the identity),
/// plus `os` and `hostname` display metadata.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "machine.MachineIdentity")]
pub struct MachineIdentity {
    /// `hex(sha256("objectiveai-machine-id" || <os machine id>))` —
    /// stable per machine, never the raw OS identifier.
    pub id: String,
    /// `std::env::consts::OS` ("windows" / "linux" / "macos" / …).
    /// Display discriminator; render unknown values generically.
    pub os: String,
    /// The machine's hostname — display label only, never identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub hostname: Option<String>,
}

/// Resolve this machine's identity. `objectiveai_dir` roots the
/// persisted-UUID fallback used when no OS identifier is readable.
#[cfg(any(feature = "lockfile", feature = "machine"))]
pub fn machine_identity(objectiveai_dir: &std::path::Path) -> MachineIdentity {
    MachineIdentity {
        id: machine_id(objectiveai_dir),
        os: std::env::consts::OS.to_string(),
        hostname: hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .filter(|h| !h.is_empty()),
    }
}

/// The hashed machine id (see module docs for the derivation chain).
#[cfg(any(feature = "lockfile", feature = "machine"))]
pub fn machine_id(objectiveai_dir: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    let raw = raw_machine_id()
        .or_else(|| persisted_machine_id(objectiveai_dir))
        // Last resort (unreadable OS id AND unwritable dir): a fresh
        // UUID — unstable across runs, but every layer treats the id
        // as opaque so nothing breaks; classification just degrades.
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut hasher = Sha256::new();
    hasher.update(b"objectiveai-machine-id");
    hasher.update(raw.trim().as_bytes());
    hex::encode(hasher.finalize())
}

/// The platform's raw installation identifier, `None` when unreadable.
#[cfg(all(any(feature = "lockfile", feature = "machine"), target_os = "linux"))]
fn raw_machine_id() -> Option<String> {
    ["/etc/machine-id", "/var/lib/dbus/machine-id"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(all(any(feature = "lockfile", feature = "machine"), target_os = "macos"))]
fn raw_machine_id() -> Option<String> {
    // `ioreg -rd1 -c IOPlatformExpertDevice` prints a line like:
    //   "IOPlatformUUID" = "XXXXXXXX-XXXX-…"
    //
    // DELIBERATE std (not tokio) subprocess — the repo-wide rule is
    // tokio-only, but `machine_identity` is a sync API consumed from
    // sync contexts, and this macOS-only one-shot read can't await.
    let output = std::process::Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().find(|l| l.contains("IOPlatformUUID"))?;
    let value = line.split('"').nth(3)?;
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(all(any(feature = "lockfile", feature = "machine"), windows))]
fn raw_machine_id() -> Option<String> {
    // HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid — the Windows
    // installation GUID. RegGetValueW with RRF_RT_REG_SZ; wide-string
    // in/out.
    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW,
    };
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
    let subkey = wide("SOFTWARE\\Microsoft\\Cryptography");
    let value = wide("MachineGuid");
    let mut buf = [0u16; 128];
    let mut size = (buf.len() * 2) as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut _,
            &mut size,
        )
    };
    if rc != 0 {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let guid = String::from_utf16_lossy(&buf[..len]);
    (!guid.trim().is_empty()).then(|| guid.trim().to_string())
}

#[cfg(all(
    any(feature = "lockfile", feature = "machine"),
    not(any(target_os = "linux", target_os = "macos", windows))
))]
fn raw_machine_id() -> Option<String> {
    None
}

/// Read-or-create the fallback UUID at `<objectiveai_dir>/bin/machine-id`.
#[cfg(any(feature = "lockfile", feature = "machine"))]
fn persisted_machine_id(objectiveai_dir: &std::path::Path) -> Option<String> {
    let path = objectiveai_dir.join("bin").join("machine-id");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let existing = existing.trim();
        if !existing.is_empty() {
            return Some(existing.to_string());
        }
    }
    let fresh = uuid::Uuid::new_v4().to_string();
    std::fs::create_dir_all(path.parent()?).ok()?;
    std::fs::write(&path, &fresh).ok()?;
    Some(fresh)
}
