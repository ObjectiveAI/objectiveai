//! Every file the app keeps, kept so it can't be lost.
//!
//! - **Versioned.** Each file says what kind of file it is and which
//!   version of it this is: `{"file": …, "version": …, "data": …}`. A file
//!   from before files carried a version is read as it is.
//! - **Whole or not at all.** A write goes to a new file beside the old one,
//!   is synced, then renamed over it. Every file is owner-only from the
//!   moment it's made.
//! - **Never written over.** A file that won't parse, that doesn't check,
//!   or that a newer version of the app wrote, is set aside under a new
//!   name beside it, untouched, and the app says so on screen. It is never
//!   replaced by defaults: where a format keeps its last good copy
//!   (`<file>.bak`), the app carries on from that; otherwise it starts that
//!   file empty, and the one set aside is still there.
//! - **Left alone when it can't be read.** A file the system won't open or
//!   read this time (no permission, a folder in its place, a failing disk)
//!   says nothing about what it holds, so it is left where it is: the app
//!   neither reads nor writes it until it starts again, and says so.
//! - **One copy of the app per folder.** The app holds its folder
//!   ([`hold`]) for as long as it runs; a second copy on the same folder
//!   changes nothing in it.

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

/// Your keys: your account, personas and your agents' keys. Version 2
/// added the account; a version 1 file reads as one with no account yet.
pub const KEYS: Format = Format { name: "keys", version: 2, keep_previous: true };
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

/// Every kind of file the app itself keeps (the stand-in keeps two more of its own).
#[allow(dead_code)] // read by the test that every kind has words on screen
pub const FORMATS: &[Format] = &[KEYS, COUNTERS, ALLOWANCES, VIEWS, MACHINE_NAMES, AGENT_MOUNTS, THREADS, RECORD_COPY];

/// Why a file can't be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// It won't parse, or it's another kind of file.
    Damaged,
    /// It parses, but what it holds doesn't check: a room's record that
    /// isn't that room's, or doesn't replay.
    Refused,
    /// A newer version of the app wrote it.
    Newer(u32),
}

/// What reading a file found.
#[derive(Debug)]
pub enum Read<T> {
    Missing,
    Good(T),
    Unusable(Why),
    /// The system wouldn't open or read it this time, in its own words.
    /// Nothing is known about what it holds, so nothing may replace it.
    Failed(String),
}

/// What was done with a file the app couldn't use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    /// Moved aside, untouched, under a new name.
    SetAside { kept_as: PathBuf, why: Why },
    /// Left where it is: the system wouldn't open or read it this time.
    LeftInPlace { error: String },
}

/// What the app went on with after a file it couldn't use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarriedOn {
    /// The last good copy, kept beside it.
    LastGood,
    /// Nothing: it started that file empty.
    Empty,
    /// What it already had open: the file went bad after the app read it.
    WhatItHad,
    /// Nothing, and it writes nothing there until it starts again.
    NothingThisLaunch,
}

/// A file the app couldn't use, and what it did about it.
#[derive(Debug, Clone)]
pub struct Notice {
    /// Where the file was.
    pub file: PathBuf,
    /// The format's file, which this is or is the last good copy of.
    pub slot: PathBuf,
    /// What kind of file it is ([`Format::name`]).
    pub kind: &'static str,
    pub done: Done,
    pub carried_on: CarriedOn,
    #[allow(dead_code)] // kept for whoever reads the list; the screen says what and where
    pub at: DateTime<Utc>,
}

impl Notice {
    /// Where it is now, when it was set aside.
    #[cfg(test)]
    pub fn kept_as(&self) -> Option<&Path> {
        match &self.done {
            Done::SetAside { kept_as, .. } => Some(kept_as),
            Done::LeftInPlace { .. } => None,
        }
    }

    /// Whether it's the last good copy kept beside a file, not the file itself.
    pub fn last_good_copy(&self) -> bool {
        self.file != self.slot
    }
}

/// Every file the app couldn't use since it started, in every folder it keeps.
static NOTICES: Mutex<Vec<Notice>> = Mutex::new(Vec::new());

/// Files left where they are because they couldn't be read: none is read
/// or written again until the app starts again.
static LEFT: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// The files the app couldn't use since it started, under one folder.
pub fn notices_under(root: &Path) -> Vec<Notice> {
    NOTICES.lock().unwrap_or_else(|p| p.into_inner()).iter().filter(|n| n.file.starts_with(root)).cloned().collect()
}

fn note(file: &Path, slot: &Path, format: Format, done: Done, carried_on: CarriedOn) {
    let notice = Notice { file: file.to_path_buf(), slot: slot.to_path_buf(), kind: format.name, done, carried_on, at: Utc::now() };
    NOTICES.lock().unwrap_or_else(|p| p.into_inner()).push(notice);
}

/// Whether a file was left where it is this launch.
pub fn is_left(slot: &Path) -> bool {
    LEFT.lock().unwrap_or_else(|p| p.into_inner()).iter().any(|p| p == slot)
}

/// Leave a format's file alone until the app starts again, because `file`
/// (it, or its last good copy) couldn't be read; say so once.
fn leave(slot: &Path, file: &Path, format: Format, error: String) {
    let mut left = LEFT.lock().unwrap_or_else(|p| p.into_inner());
    if left.iter().any(|p| p == slot) {
        return;
    }
    left.push(slot.to_path_buf());
    drop(left);
    note(file, slot, format, Done::LeftInPlace { error }, CarriedOn::NothingThisLaunch);
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
        Err(e) => Read::Failed(e.to_string()),
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

/// The version a kept file says it is, without reading what it holds: 0
/// for one from before files carried a version; `None` if it won't read.
pub fn version_on_disk(path: &Path) -> Option<u32> {
    let value: Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    match value.as_object() {
        Some(o) if o.contains_key("file") && o.contains_key("version") => o.get("version").and_then(Value::as_u64).and_then(|v| u32::try_from(v).ok()),
        _ => Some(0),
    }
}

/// Keep a copy of a file under another name, owner-only and synced, once:
/// a copy already there is left as it is.
pub fn keep_copy(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        return Ok(());
    }
    write_new(to, &std::fs::read(from)?)?;
    sync_dir(to);
    Ok(())
}

/// Move a file out of the way, untouched, under a name that says why.
pub fn set_aside(path: &Path, why: &Why) -> std::io::Result<PathBuf> {
    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let tag = match why {
        Why::Damaged => format!("damaged-{stamp}"),
        Why::Refused => format!("refused-{stamp}"),
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
    Ok(kept_as)
}

/// [`set_aside`], and say so on screen.
pub fn set_aside_noting(path: &Path, slot: &Path, format: Format, why: Why, carried_on: CarriedOn) -> std::io::Result<PathBuf> {
    let kept_as = set_aside(path, &why)?;
    note(path, slot, format, Done::SetAside { kept_as: kept_as.clone(), why }, carried_on);
    Ok(kept_as)
}

/// A file the app keeps, or its last good copy when the file can't be used.
/// Whatever can't be used is set aside first: it won't parse, a newer
/// version of the app wrote it, or `check` refuses what it holds. One the
/// system won't read this time is left where it is, and nothing is read
/// or written there until the app starts again. `None`: there's nothing
/// usable to read, and nothing was written over.
pub fn load_with<T: DeserializeOwned>(path: &Path, format: Format, check: impl Fn(&T) -> Result<(), String>) -> Option<T> {
    if is_left(path) {
        return None;
    }
    let mut candidates = vec![path.to_path_buf()];
    if format.keep_previous {
        candidates.push(previous_of(path));
    }
    let mut aside = Vec::new();
    let mut found = None;
    let mut left = false;
    for candidate in candidates {
        let why = match read::<T>(&candidate, format) {
            Read::Missing => continue,
            Read::Good(t) => match check(&t) {
                Ok(()) => {
                    found = Some(t);
                    break;
                }
                Err(_) => Why::Refused,
            },
            Read::Unusable(why) => why,
            Read::Failed(error) => {
                leave(path, &candidate, format, error);
                left = true;
                break;
            }
        };
        match set_aside(&candidate, &why) {
            Ok(kept_as) => aside.push((candidate, kept_as, why)),
            Err(e) => {
                // It can't even be moved: leave it, and everything else here, alone.
                leave(path, &candidate, format, e.to_string());
                left = true;
                break;
            }
        }
    }
    let carried_on = match (left, &found) {
        (true, _) => CarriedOn::NothingThisLaunch,
        (false, Some(_)) => CarriedOn::LastGood,
        (false, None) => CarriedOn::Empty,
    };
    for (file, kept_as, why) in aside {
        note(&file, path, format, Done::SetAside { kept_as, why }, carried_on);
    }
    if left { None } else { found }
}

/// [`load_with`], taking whatever parses.
pub fn load<T: DeserializeOwned>(path: &Path, format: Format) -> Option<T> {
    load_with(path, format, |_| Ok(()))
}

/// A file the app keeps, or its last good copy, touching nothing and
/// noting nothing: for a copy of the app that doesn't hold its folder.
pub fn peek_with<T: DeserializeOwned>(path: &Path, format: Format, check: impl Fn(&T) -> Result<(), String>) -> Option<T> {
    let mut candidates = vec![path.to_path_buf()];
    if format.keep_previous {
        candidates.push(previous_of(path));
    }
    candidates.into_iter().find_map(|c| match read::<T>(&c, format) {
        Read::Good(t) if check(&t).is_ok() => Some(t),
        _ => None,
    })
}

/// [`peek_with`], taking whatever parses.
pub fn peek<T: DeserializeOwned>(path: &Path, format: Format) -> Option<T> {
    peek_with(path, format, |_| Ok(()))
}

static TEMP: AtomicU64 = AtomicU64::new(0);

fn temp_beside(path: &Path) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!(".{name}.{}-{}.tmp", std::process::id(), TEMP.fetch_add(1, Ordering::Relaxed)))
}

/// Open a file only its owner can read, made new or (`existing`) as it is.
fn owner_only(path: &Path, existing: bool) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    if existing {
        options.read(true).write(true).create(true).truncate(false);
    } else {
        options.write(true).create_new(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// A new file only its owner can read, its bytes synced to disk.
fn write_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut f = owner_only(path, false)?;
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
/// be used is set aside first; if it can't be, nothing is written. Nothing
/// is written over a file the system won't read: that one is left where it
/// is until the app starts again.
pub fn save<T: Serialize + DeserializeOwned>(path: &Path, format: Format, data: &T) -> std::io::Result<()> {
    if is_left(path) {
        return Err(std::io::Error::other("left where it is until the app starts again"));
    }
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
                set_aside_noting(path, path, format, why, CarriedOn::WhatItHad)?;
            }
            Read::Failed(error) => {
                leave(path, path, format, error.clone());
                return Err(std::io::Error::other(error));
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

/// The file in a folder that a copy of the app holds while it uses it.
pub const IN_USE_FILE: &str = ".in-use";

/// What a copy of the app that doesn't hold its folder answers, because another copy does.
/// The same words are the screen's, in `src/strings.ts`.
pub const IN_USE: &str = "Another copy of this app is using this folder, so this one changes nothing in it: nothing is saved or sent from here.";
/// What it answers when it couldn't tell whether another copy does.
/// The same words are the screen's, in `src/strings.ts`.
pub const UNCHECKED: &str = "This copy of the app couldn't make sure no other copy is using this folder, so it changes nothing in it: nothing is saved or sent from here.";

/// Whether this copy of the app holds its folder.
#[derive(Debug)]
pub enum Hold {
    /// It does, for as long as this stays open. The system lets go when the
    /// app exits, however it exits, so nothing is left holding it.
    Held(#[allow(dead_code)] std::fs::File),
    /// Another copy of the app holds it.
    Elsewhere,
    /// The system couldn't say, in its own words.
    Unchecked(String),
}

impl Hold {
    pub fn held(&self) -> bool {
        matches!(self, Hold::Held(_))
    }

    /// What this copy answers when it doesn't hold its folder.
    pub fn refusal(&self) -> Option<&'static str> {
        match self {
            Hold::Held(_) => None,
            Hold::Elsewhere => Some(IN_USE),
            Hold::Unchecked(_) => Some(UNCHECKED),
        }
    }
}

/// Hold a folder for this copy of the app: one copy at a time, on every
/// system the app runs on.
pub fn hold(dir: &Path) -> Hold {
    if let Err(e) = std::fs::create_dir_all(dir) {
        return Hold::Unchecked(e.to_string());
    }
    let file = match owner_only(&dir.join(IN_USE_FILE), true) {
        Ok(f) => f,
        Err(e) => return Hold::Unchecked(e.to_string()),
    };
    match file.try_lock() {
        Ok(()) => Hold::Held(file),
        Err(std::fs::TryLockError::WouldBlock) => Hold::Elsewhere,
        Err(std::fs::TryLockError::Error(e)) => Hold::Unchecked(e.to_string()),
    }
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

    /// The login a test's child process runs under: distinctive, so a file
    /// that holds it can only have read it from the login.
    pub const LOGIN: &str = "zz-login-probe";
    const LOGIN_CHILD: &str = "DIVERGE_DESKTOP_LOGIN_PROBE_CHILD";

    /// Run one test again in a child process whose login is [`LOGIN`]
    /// (`USER`, `LOGNAME` and `USERNAME` all set to it), so the test sees a
    /// known login wherever it runs. True in that child, where the test goes
    /// on; false in the parent once the child has run that test and passed.
    /// `module` is the test's `module_path!()`, `test` its function's name.
    pub fn under_a_known_login(module: &str, test: &str) -> bool {
        if std::env::var_os(LOGIN_CHILD).is_some() {
            for name in ["USER", "LOGNAME", "USERNAME"] {
                assert_eq!(std::env::var(name).as_deref(), Ok(LOGIN), "{name} is the login this test set");
            }
            return true;
        }
        let path = module.split_once("::").map_or(module, |(_, rest)| rest);
        let name = format!("{path}::{test}");
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([name.as_str(), "--exact", "--nocapture", "--test-threads=1"])
            .env(LOGIN_CHILD, "1")
            .env("USER", LOGIN)
            .env("LOGNAME", LOGIN)
            .env("USERNAME", LOGIN)
            .output()
            .unwrap();
        let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        assert!(out.status.success(), "{name}, under a known login, failed:\n{said}");
        assert!(said.contains("1 passed"), "{name} ran in the child:\n{said}");
        false
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
        let aside = notices_under(&dir);
        assert_eq!(aside.len(), 1);
        assert_eq!((aside[0].file.as_path(), aside[0].kind, &aside[0].carried_on), (file.as_path(), "notes", &CarriedOn::LastGood));
        assert!(matches!(&aside[0].done, Done::SetAside { why: Why::Damaged, .. }), "{:?}", aside[0].done);
        assert!(!aside[0].last_good_copy());
        assert_eq!(std::fs::read(aside[0].kept_as().unwrap()).unwrap(), &whole[..whole.len() / 2], "kept exactly as it was");
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
        let aside = notices_under(&dir);
        assert_eq!(aside.len(), 1);
        assert_eq!((&aside[0].done, aside[0].carried_on), (&Done::SetAside { kept_as: aside[0].kept_as().unwrap().to_path_buf(), why: Why::Newer(7) }, CarriedOn::Empty));
        save(&file, PLAIN, &notes(1)).unwrap();
        assert_eq!(std::fs::read_to_string(aside[0].kept_as().unwrap()).unwrap(), newer, "still there, untouched");
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
        let aside = notices_under(&dir);
        assert_eq!(aside.len(), 1);
        assert_eq!(aside[0].carried_on, CarriedOn::WhatItHad, "the app kept what it had open");
        assert_eq!(std::fs::read_to_string(aside[0].kept_as().unwrap()).unwrap(), "{ \"file\": \"notes\", \"vers");
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
        let aside = notices_under(&dir);
        assert_eq!(aside.len(), 1);
        assert!(matches!(&aside[0].done, Done::SetAside { why: Why::Refused, .. }), "it parsed, but didn't check: {:?}", aside[0].done);
        assert_eq!(aside[0].carried_on, CarriedOn::LastGood);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_the_system_wont_read_is_left_where_it_is_and_nothing_replaces_it() {
        let dir = folder("store-unread");
        // A folder where the file should be: the system won't read it as a file.
        let file = dir.join("notes.json");
        std::fs::create_dir_all(file.join("inside")).unwrap();
        std::fs::write(previous_of(&file), r#"{"file":"notes","version":2,"data":{"note 0":0}}"#).unwrap();
        let before = names_in(&dir);
        assert!(matches!(read::<BTreeMap<String, u32>>(&file, NOTES), Read::Failed(_)));
        assert_eq!(load::<BTreeMap<String, u32>>(&file, NOTES), None, "nothing read this launch, not even the last good copy");
        assert!(save(&file, NOTES, &notes(3)).is_err(), "and nothing written there");
        assert_eq!(names_in(&dir), before, "nothing moved, nothing made");
        assert!(file.join("inside").is_dir());
        let aside = notices_under(&dir);
        assert_eq!(aside.len(), 1, "said once: {aside:?}");
        assert!(matches!(&aside[0].done, Done::LeftInPlace { .. }));
        assert_eq!((aside[0].kept_as(), aside[0].carried_on), (None, CarriedOn::NothingThisLaunch));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_last_good_copy_the_system_wont_read_keeps_the_file_from_being_written() {
        let dir = folder("store-unread-backup");
        let file = dir.join("notes.json");
        std::fs::write(&file, "{ damaged").unwrap();
        std::fs::create_dir_all(previous_of(&file).join("inside")).unwrap();
        assert_eq!(load::<BTreeMap<String, u32>>(&file, NOTES), None);
        assert!(save(&file, NOTES, &notes(3)).is_err(), "a later write could push it out, so there's none");
        assert!(previous_of(&file).join("inside").is_dir());
        let aside = notices_under(&dir);
        assert_eq!(aside.len(), 2, "{aside:?}");
        assert!(aside.iter().all(|n| n.carried_on == CarriedOn::NothingThisLaunch));
        assert!(aside.iter().any(|n| n.last_good_copy() && matches!(n.done, Done::LeftInPlace { .. })));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_peek_touches_nothing() {
        let dir = folder("store-peek");
        let file = dir.join("notes.json");
        save(&file, NOTES, &notes(2)).unwrap();
        save(&file, NOTES, &notes(5)).unwrap();
        std::fs::write(&file, "{ damaged").unwrap();
        let before = names_in(&dir);
        assert_eq!(peek::<BTreeMap<String, u32>>(&file, NOTES), Some(notes(2)), "the last good copy");
        assert_eq!(names_in(&dir), before);
        assert!(notices_under(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_kind_of_file_has_words_on_screen() {
        let screen = include_str!("../../src/strings.ts");
        // Only the stand-in adds to the list.
        #[cfg_attr(not(feature = "stand-in"), allow(unused_mut))]
        let mut formats = FORMATS.to_vec();
        #[cfg(feature = "stand-in")]
        formats.extend([crate::spaces::stub::ROOMS, crate::daemon::stub::store::VOLUMES]);
        for f in formats {
            assert!(screen.contains(&format!("\"{}\": \"", f.name)), "src/strings.ts words the {} file", f.name);
        }
    }

    #[test]
    fn one_copy_of_the_app_holds_a_folder_at_a_time() {
        let dir = folder("store-hold");
        let first = hold(&dir);
        assert!(first.held(), "{first:?}");
        assert!(matches!(hold(&dir), Hold::Elsewhere), "a second copy finds it held");
        assert_eq!(hold(&dir).refusal(), Some(IN_USE));
        drop(first);
        assert!(hold(&dir).held(), "let go when the first one closes");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(dir.join(IN_USE_FILE)).unwrap().permissions().mode() & 0o777, 0o600, "owner-only");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
