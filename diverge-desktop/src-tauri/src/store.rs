//! Every file the app keeps, kept so it can't be lost.
//!
//! - **Versioned.** Each file says what kind of file it is and which
//!   version of it this is: `{"file": …, "version": …, "data": …}`. A file
//!   from before files carried a version is read as it is.
//! - **Whole or not at all.** A write goes to a new file beside the old one,
//!   is synced, then renamed over it. Every file is owner-only from the
//!   moment it's made.
//! - **Never written over.** A file that won't parse, or that a newer
//!   version of the app wrote, is set aside under a new name beside it,
//!   untouched, and the app says so on screen. It is never replaced by
//!   defaults: where a format keeps its last good copy (`<file>.bak`), the
//!   app carries on from that; otherwise it starts that file empty, and
//!   the one set aside is still there.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
#[cfg(test)]
use serde::Deserialize;
use serde_json::Value;

/// One kind of file the app keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    /// What kind of file it is, written in it so it can't be taken for another.
    pub name: &'static str,
    /// The newest version this build reads and writes.
    pub version: u32,
    /// Whether the last good copy is kept beside it, to carry on from.
    pub keep_previous: bool,
}

/// Your keys: personas and your agents' keys.
pub const KEYS: Format = Format { name: "keys", version: 1, keep_previous: true };
/// Each key's last counter. Never behind the clock, so losing it locks nobody out.
pub const COUNTERS: Format = Format { name: "counters", version: 1, keep_previous: false };
/// Your agents' allowances, room by room.
pub const ALLOWANCES: Format = Format { name: "allowances", version: 1, keep_previous: true };
/// Saved Views.
pub const VIEWS: Format = Format { name: "views", version: 1, keep_previous: true };
/// The names you gave your machines.
pub const MACHINE_NAMES: Format = Format { name: "machine names", version: 1, keep_previous: true };
/// What the app last stated each agent mounts.
pub const AGENT_MOUNTS: Format = Format { name: "agent mounts", version: 1, keep_previous: true };
/// Your asks' thread ids, and the one thread each belongs to.
pub const THREADS: Format = Format { name: "threads", version: 1, keep_previous: true };
/// Your copy of one room's record.
pub const RECORD_COPY: Format = Format { name: "record copy", version: 1, keep_previous: true };

/// Why a file was set aside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// It won't parse, it's another kind of file, or what it holds doesn't check.
    Damaged,
    /// A newer version of the app wrote it.
    Newer(u32),
}

/// What reading a file found.
#[derive(Debug)]
pub enum Read<T> {
    Missing,
    Good(T),
    Unusable(Why),
}

/// A file set aside: where it was, where it is now, and why.
#[derive(Debug, Clone)]
pub struct SetAside {
    pub file: PathBuf,
    pub kept_as: PathBuf,
    pub why: Why,
    #[allow(dead_code)] // kept for whoever reads the list; the screen says what and where
    pub at: DateTime<Utc>,
}

/// Every file set aside since the app started, in every folder it keeps.
static SET_ASIDE: Mutex<Vec<SetAside>> = Mutex::new(Vec::new());

/// The files set aside since the app started, under one folder.
pub fn set_aside_under(root: &Path) -> Vec<SetAside> {
    SET_ASIDE.lock().unwrap_or_else(|p| p.into_inner()).iter().filter(|s| s.kept_as.starts_with(root)).cloned().collect()
}

/// Where a file's last good copy is kept.
pub fn previous_of(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

#[derive(Serialize)]
struct Out<'a, T> {
    file: &'a str,
    version: u32,
    data: &'a T,
}

/// Read a file as `format`, touching nothing.
pub fn read<T: DeserializeOwned>(path: &Path, format: Format) -> Read<T> {
    match std::fs::read(path) {
        Ok(bytes) => parse(&bytes, format),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Read::Missing,
        Err(_) => Read::Unusable(Why::Damaged),
    }
}

fn parse<T: DeserializeOwned>(bytes: &[u8], format: Format) -> Read<T> {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else { return Read::Unusable(Why::Damaged) };
    let wrapped = value.as_object().is_some_and(|o| o.contains_key("file") && o.contains_key("version"));
    if !wrapped {
        // From before files carried a version: read as it is.
        return match serde_json::from_value(value) {
            Ok(t) => Read::Good(t),
            Err(_) => Read::Unusable(Why::Damaged),
        };
    }
    let Value::Object(mut o) = value else { return Read::Unusable(Why::Damaged) };
    if o.get("file").and_then(Value::as_str) != Some(format.name) {
        return Read::Unusable(Why::Damaged);
    }
    let Some(version) = o.get("version").and_then(Value::as_u64) else { return Read::Unusable(Why::Damaged) };
    if version > u64::from(format.version) {
        return Read::Unusable(Why::Newer(u32::try_from(version).unwrap_or(u32::MAX)));
    }
    match o.remove("data").map(serde_json::from_value) {
        Some(Ok(t)) => Read::Good(t),
        _ => Read::Unusable(Why::Damaged),
    }
}

/// Move a file out of the way, untouched, under a name that says why, and
/// note it for the screen.
pub fn set_aside(path: &Path, why: &Why) -> std::io::Result<PathBuf> {
    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let tag = match why {
        Why::Damaged => format!("damaged-{stamp}"),
        Why::Newer(v) => format!("newer-v{v}-{stamp}"),
    };
    let mut n = 0u32;
    let kept_as = loop {
        let candidate = path.with_extension(if n == 0 { format!("{tag}.json") } else { format!("{tag}-{n}.json") });
        if !candidate.exists() {
            break candidate;
        }
        n += 1;
    };
    std::fs::rename(path, &kept_as)?;
    sync_dir(path);
    SET_ASIDE.lock().unwrap_or_else(|p| p.into_inner()).push(SetAside { file: path.to_path_buf(), kept_as: kept_as.clone(), why: why.clone(), at: Utc::now() });
    Ok(kept_as)
}

/// A file the app keeps, or its last good copy when the file can't be used.
/// Whatever can't be used is set aside first: it won't parse, a newer
/// version of the app wrote it, or `check` refuses what it holds. `None`:
/// there's nothing usable to read, and nothing was written over.
pub fn load_with<T: DeserializeOwned>(path: &Path, format: Format, check: impl Fn(&T) -> Result<(), String>) -> Option<T> {
    let mut candidates = vec![path.to_path_buf()];
    if format.keep_previous {
        candidates.push(previous_of(path));
    }
    for candidate in candidates {
        match read::<T>(&candidate, format) {
            Read::Missing => {}
            Read::Good(t) if check(&t).is_ok() => return Some(t),
            Read::Good(_) => {
                let _ = set_aside(&candidate, &Why::Damaged);
            }
            Read::Unusable(why) => {
                let _ = set_aside(&candidate, &why);
            }
        }
    }
    None
}

/// [`load_with`], taking whatever parses.
pub fn load<T: DeserializeOwned>(path: &Path, format: Format) -> Option<T> {
    load_with(path, format, |_| Ok(()))
}

static TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_beside(path: &Path) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!(".{name}.{}-{}.tmp", std::process::id(), TEMP.fetch_add(1, Ordering::Relaxed)))
}

/// A new file only its owner can read, its bytes synced to disk.
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Sync the folder a file is in, so a rename in it survives a crash.
fn sync_dir(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Write a file the app keeps, whole or not at all. The one it replaces
/// becomes the last good copy, for a format that keeps one. One that can't
/// be used is set aside first; if it can't be, nothing is written.
pub fn save<T: Serialize + DeserializeOwned>(path: &Path, format: Format, data: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(&Out { file: format.name, version: format.version, data }).map_err(std::io::Error::other)?;
    let tmp = temp_beside(path);
    write_new(&tmp, &bytes)?;
    let swapped = (|| {
        match read::<T>(path, format) {
            Read::Missing => {}
            Read::Unusable(why) => {
                set_aside(path, &why)?;
            }
            Read::Good(_) if format.keep_previous => match std::fs::rename(path, previous_of(path)) {
                Ok(()) => {}
                // Someone else's write moved it a moment ago.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            },
            Read::Good(_) => {}
        }
        std::fs::rename(&tmp, path)?;
        sync_dir(path);
        Ok(())
    })();
    if swapped.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    swapped
}

/// What a kept file says about itself, without reading what it holds.
#[cfg(test)]
#[derive(Debug, Deserialize)]
pub struct Header {
    pub file: String,
    pub version: u32,
}

/// The kind and version a kept file carries, if it carries them.
#[cfg(test)]
pub fn header(path: &Path) -> Option<Header> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const NOTES: Format = Format { name: "notes", version: 2, keep_previous: true };
    const PLAIN: Format = Format { name: "plain", version: 1, keep_previous: false };

    /// A fresh folder of its own for one test.
    pub fn folder(what: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("diverge-desktop-{what}-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn notes(n: u32) -> BTreeMap<String, u32> {
        (0..n).map(|i| (format!("note {i}"), i)).collect()
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    #[test]
    fn a_file_carries_its_kind_and_version_and_reads_back() {
        let dir = folder("store-roundtrip");
        let file = dir.join("notes.json");
        save(&file, NOTES, &notes(3)).unwrap();
        let h = header(&file).unwrap();
        assert_eq!((h.file.as_str(), h.version), ("notes", 2));
        assert_eq!(load::<BTreeMap<String, u32>>(&file, NOTES), Some(notes(3)));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600, "owner-only");
        }
        assert!(!names_in(&dir).iter().any(|n| n.ends_with(".tmp")), "nothing left half-written: {:?}", names_in(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_from_before_versions_reads_as_it_is_and_gains_one_when_written() {
        let dir = folder("store-unversioned");
        let file = dir.join("notes.json");
        std::fs::write(&file, serde_json::to_string(&notes(2)).unwrap()).unwrap();
        assert_eq!(load::<BTreeMap<String, u32>>(&file, NOTES), Some(notes(2)));
        save(&file, NOTES, &notes(4)).unwrap();
        assert_eq!(header(&file).unwrap().version, 2);
        assert_eq!(load::<BTreeMap<String, u32>>(&previous_of(&file), NOTES), Some(notes(2)), "the old one is the last good copy");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cut_short_file_is_set_aside_and_the_last_good_one_carries_on() {
        let dir = folder("store-truncated");
        let file = dir.join("notes.json");
        save(&file, NOTES, &notes(2)).unwrap();
        save(&file, NOTES, &notes(5)).unwrap();
        let whole = std::fs::read(&file).unwrap();
        std::fs::write(&file, &whole[..whole.len() / 2]).unwrap();
        assert_eq!(load::<BTreeMap<String, u32>>(&file, NOTES), Some(notes(2)), "the last good copy");
        let aside = set_aside_under(&dir);
        assert_eq!(aside.len(), 1);
        assert_eq!((aside[0].file.as_path(), &aside[0].why), (file.as_path(), &Why::Damaged));
        assert_eq!(std::fs::read(&aside[0].kept_as).unwrap(), &whole[..whole.len() / 2], "kept exactly as it was");
        assert!(!file.exists(), "moved, not copied");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_from_a_newer_version_is_set_aside_never_written_over() {
        let dir = folder("store-newer");
        let file = dir.join("plain.json");
        let newer = r#"{"file":"plain","version":7,"data":{"shape":"unknown to this build"}}"#;
        std::fs::write(&file, newer).unwrap();
        assert!(matches!(read::<BTreeMap<String, u32>>(&file, PLAIN), Read::Unusable(Why::Newer(7))));
        assert_eq!(load::<BTreeMap<String, u32>>(&file, PLAIN), None, "nothing this build can use");
        let aside = set_aside_under(&dir);
        assert_eq!(aside.len(), 1);
        assert_eq!(aside[0].why, Why::Newer(7));
        save(&file, PLAIN, &notes(1)).unwrap();
        assert_eq!(std::fs::read_to_string(&aside[0].kept_as).unwrap(), newer, "still there, untouched");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_write_sets_aside_what_it_would_replace_if_that_cant_be_read() {
        let dir = folder("store-write-over");
        let file = dir.join("notes.json");
        save(&file, NOTES, &notes(1)).unwrap();
        save(&file, NOTES, &notes(2)).unwrap();
        // Damaged after the app read it: the next write keeps it, and keeps the last good copy too.
        std::fs::write(&file, "{ \"file\": \"notes\", \"vers").unwrap();
        save(&file, NOTES, &notes(3)).unwrap();
        assert_eq!(load::<BTreeMap<String, u32>>(&file, NOTES), Some(notes(3)));
        assert_eq!(load::<BTreeMap<String, u32>>(&previous_of(&file), NOTES), Some(notes(1)), "the damaged one never became the last good copy");
        let aside = set_aside_under(&dir);
        assert_eq!(aside.len(), 1);
        assert_eq!(std::fs::read_to_string(&aside[0].kept_as).unwrap(), "{ \"file\": \"notes\", \"vers");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn another_kind_of_file_and_one_that_doesnt_check_are_set_aside() {
        let dir = folder("store-kind");
        let file = dir.join("notes.json");
        save(&file, PLAIN, &notes(1)).unwrap();
        assert!(matches!(read::<BTreeMap<String, u32>>(&file, NOTES), Read::Unusable(Why::Damaged)), "a plain file isn't notes");
        let file = dir.join("checked.json");
        save(&file, NOTES, &notes(1)).unwrap();
        save(&file, NOTES, &notes(9)).unwrap();
        let checked = load_with::<BTreeMap<String, u32>>(&file, NOTES, |n| if n.len() > 5 { Err("too many".into()) } else { Ok(()) });
        assert_eq!(checked, Some(notes(1)));
        assert_eq!(set_aside_under(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
