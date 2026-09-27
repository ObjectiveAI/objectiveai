//! One stand-in machine's volumes: real folders on this Mac, one per
//! volume, under that machine's folder in the stand-in's data folder.
//! The provider protocol's `volumes::*` answer from here with its own
//! types. Nothing outside a volume's folder is ever touched.

use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use diverge_sdk::provider::endpoints::volumes::Mode;
use diverge_sdk::provider::endpoints::volumes::list::server::response::Volume;
use diverge_sdk::provider::endpoints::volumes::stat::server::response::Stat;
use diverge_sdk::shared::filetree::response::Node;

/// What each stand-in machine has to give, in all.
pub const TOTAL: u64 = 64 << 30;

#[derive(Serialize, Deserialize, Clone)]
struct Meta {
    bytes: u64,
    created: u64,
    mode: Mode,
}

pub struct VolumeStore {
    root: PathBuf,
    meta: Mutex<BTreeMap<String, Meta>>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A daemon path ("/projects/site/index.html") as the wire's components.
pub fn components(path: &str) -> Vec<String> {
    path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect()
}

impl VolumeStore {
    pub fn new(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(root.join("volumes"));
        let meta: BTreeMap<String, Meta> = fs::read_to_string(root.join("volumes.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        VolumeStore { root, meta: Mutex::new(meta) }
    }

    fn save(&self, meta: &BTreeMap<String, Meta>) {
        if let Ok(json) = serde_json::to_string_pretty(meta) {
            let _ = fs::write(self.root.join("volumes.json"), json);
        }
    }

    fn dir(&self, name: &str) -> PathBuf {
        self.root.join("volumes").join(name)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.meta.lock().unwrap().contains_key(name)
    }

    pub fn mode(&self, name: &str) -> Option<Mode> {
        self.meta.lock().unwrap().get(name).map(|m| m.mode)
    }

    fn resolve(&self, name: &str, path: &[String]) -> Result<PathBuf, String> {
        if !self.exists(name) {
            return Err(format!("no volume named \"{name}\""));
        }
        let mut out = self.dir(name);
        for part in path {
            match Path::new(part).components().next() {
                Some(Component::Normal(_)) if !part.contains('/') => out.push(part),
                _ => return Err(format!("\"{part}\" is not a name inside a volume")),
            }
        }
        Ok(out)
    }

    fn used(&self, name: &str) -> u64 {
        fn walk(p: &Path) -> u64 {
            let Ok(read) = fs::read_dir(p) else { return 0 };
            read.flatten().map(|e| match e.metadata() { Ok(m) if m.is_dir() => walk(&e.path()), Ok(m) => m.len(), Err(_) => 0 }).sum()
        }
        walk(&self.dir(name))
    }

    pub fn list(&self) -> Vec<Volume> {
        self.meta.lock().unwrap().iter().map(|(name, m)| Volume { name: name.clone(), bytes: m.bytes, created: m.created, mode: m.mode }).collect()
    }

    pub fn stat(&self, name: &str) -> Result<Stat, String> {
        let m = self.meta.lock().unwrap().get(name).cloned().ok_or_else(|| format!("no volume named \"{name}\""))?;
        let tree = self.tree(name, &[])?;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{tree:?}").hash(&mut h);
        Ok(Stat {
            volume: Volume { name: name.to_owned(), bytes: m.bytes, created: m.created, mode: m.mode },
            bytes_used: self.used(name),
            dirhash: format!("{:016x}", h.finish()),
        })
    }

    pub fn read(&self, name: &str, path: &[String]) -> Result<Vec<u8>, String> {
        let p = self.resolve(name, path)?;
        if p.is_dir() {
            return Err(format!("/{} is a folder", path.join("/")));
        }
        fs::read(&p).map_err(|_| format!("nothing at /{} in {name}", path.join("/")))
    }

    /// Put a file in place whole: written beside, then renamed over.
    pub fn write(&self, name: &str, path: &[String], body: &[u8]) -> Result<(), String> {
        let p = self.resolve(name, path)?;
        let parent = p.parent().ok_or("no folder")?;
        if !parent.is_dir() {
            return Err(format!("the folder for /{} does not exist in {name}", path.join("/")));
        }
        let size = self.meta.lock().unwrap().get(name).map(|m| m.bytes).unwrap_or(0);
        let current = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        if self.used(name) - current + body.len() as u64 > size {
            return Err(format!("{name} is full"));
        }
        let temp = parent.join(format!(".{}.writing", p.file_name().unwrap_or_default().to_string_lossy()));
        fs::write(&temp, body).map_err(|e| e.to_string())?;
        fs::rename(&temp, &p).map_err(|e| e.to_string())
    }

    /// One snapshot of a subtree, as the wire names it.
    pub fn tree(&self, name: &str, path: &[String]) -> Result<Vec<Node>, String> {
        let p = self.resolve(name, path)?;
        if !p.is_dir() {
            return Err(format!("/{} is not a folder in {name}", path.join("/")));
        }
        Ok(entries(&p))
    }

    fn reserved(&self) -> u64 {
        self.meta.lock().unwrap().values().map(|m| m.bytes).sum()
    }

    pub fn create_capacity(&self) -> u64 {
        TOTAL.saturating_sub(self.reserved())
    }

    pub fn edit_capacity(&self, name: &str) -> Result<u64, String> {
        let m = self.meta.lock().unwrap().get(name).cloned().ok_or_else(|| format!("no volume named \"{name}\""))?;
        Ok(self.create_capacity() + m.bytes)
    }

    /// `Ok(true)` created, `Ok(false)` not enough room.
    pub fn create(&self, name: &str, bytes: u64, mode: Mode) -> Result<bool, String> {
        let name = name.trim();
        if name.is_empty() || name.contains('/') || name.starts_with('.') {
            return Err("a volume needs a plain name".into());
        }
        if self.exists(name) {
            return Err(format!("there is already a volume named \"{name}\""));
        }
        if bytes > self.create_capacity() {
            return Ok(false);
        }
        fs::create_dir_all(self.dir(name)).map_err(|e| e.to_string())?;
        let mut meta = self.meta.lock().unwrap();
        meta.insert(name.to_owned(), Meta { bytes, created: now(), mode });
        self.save(&meta);
        Ok(true)
    }

    /// `Ok(None)` edited; `Ok(Some("room"))` / `Ok(Some("content"))` refused.
    pub fn edit(&self, name: &str, bytes: Option<u64>, mode: Option<Mode>) -> Result<Option<&'static str>, String> {
        let capacity = self.edit_capacity(name)?;
        let used = self.used(name);
        let mut meta = self.meta.lock().unwrap();
        let m = meta.get_mut(name).ok_or_else(|| format!("no volume named \"{name}\""))?;
        if let Some(bytes) = bytes {
            if bytes > capacity {
                return Ok(Some("room"));
            }
            if bytes < used {
                return Ok(Some("content"));
            }
            m.bytes = bytes;
        }
        if let Some(mode) = mode {
            m.mode = mode;
        }
        self.save(&meta);
        Ok(None)
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let mut meta = self.meta.lock().unwrap();
        if meta.remove(name).is_none() {
            return Err(format!("no volume named \"{name}\""));
        }
        let _ = fs::remove_dir_all(self.dir(name));
        self.save(&meta);
        Ok(())
    }

    /// Read a text file for a script, by daemon-style path, from `workspace`.
    pub fn read_text(&self, path: &str) -> String {
        self.read("workspace", &components(path)).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
    }

    /// This Mac's machine: a workspace that keeps changes, datasets agents
    /// may only read, and scratch that starts fresh every run.
    pub fn seed_here(&self) {
        self.seed(&[
            ("workspace", 8 << 30, Mode::Persistent, &[
                ("projects/site/index.html", SITE_INDEX),
                ("projects/site/styles.css", SITE_CSS),
                ("projects/site/README.md", SITE_README),
                ("notes/ideas.md", NOTES_IDEAS),
                ("notes/reading-list.md", NOTES_READING),
            ]),
            ("datasets", 32 << 30, Mode::ReadOnly, &[("README.md", "# datasets\n\nNothing here yet.\n")]),
            ("scratch", 1 << 30, Mode::Ephemeral, &[]),
        ]);
    }

    /// The studio PC: one volume of renders, kept.
    pub fn seed_studio(&self) {
        self.seed(&[("media", 16 << 30, Mode::Persistent, &[("README.md", "# media\n\nRenders and references from the studio PC.\n")])]);
    }

    fn seed(&self, seeds: &[(&str, u64, Mode, &[(&str, &str)])]) {
        for (name, bytes, mode, files) in seeds {
            if self.exists(name) {
                continue;
            }
            let _ = self.create(name, *bytes, *mode);
            for (rel, body) in *files {
                let p = self.dir(name).join(rel);
                if let Some(parent) = p.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(p, body);
            }
        }
    }
}

fn secs(time: std::io::Result<SystemTime>) -> Option<u64> {
    time.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs())
}

fn entries(dir: &Path) -> Vec<Node> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    let mut nodes: Vec<Node> = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(entry.path()) else { continue };
        let (created_at, modified_at) = (secs(meta.created()), secs(meta.modified()));
        if meta.file_type().is_symlink() {
            nodes.push(Node::Symlink { name, path: Vec::new(), created_at, modified_at });
        } else if meta.is_dir() {
            // A snapshot: nothing watches, so `changes` is false (the wire's rule).
            nodes.push(Node::Directory { name, created_at, modified_at, changes: false, children: entries(&entry.path()) });
        } else {
            nodes.push(Node::File { name, size: Some(meta.len()), created_at, modified_at });
        }
    }
    nodes.sort_by(|a, b| name_of(a).cmp(name_of(b)));
    nodes
}

pub fn name_of(node: &Node) -> &str {
    match node {
        Node::File { name, .. } | Node::Directory { name, .. } | Node::Symlink { name, .. } => name,
    }
}

const SITE_INDEX: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Workshop</title>
    <link rel="stylesheet" href="styles.css" />
  </head>
  <body>
    <header><h1>Workshop</h1></header>
    <main>
      <p>Things I'm building, and how to reach me about them.</p>
      <a href="/projects">Projects</a>
      <a href="/notes">Notes</a>
      <a href="mailto:hello@example.com">Email</a>
      <a href="/old-page">Archive</a>
    </main>
  </body>
</html>
"#;
const SITE_CSS: &str = "body { font-family: system-ui; margin: 0 auto; max-width: 42rem; }\nheader h1 { font-size: 2rem; }\n";
const SITE_README: &str = "# site\n\nA small static site. `index.html` and `styles.css`, nothing else.\n";
const NOTES_IDEAS: &str = "# ideas\n\n- Fix the Archive link on the site\n- Ask ada about the studio PC on Saturday\n- A darker theme for the site\n- Sort the trip photos\n";
const NOTES_READING: &str = "# reading list\n\n- The provider protocol, 2.3.0\n- Notes on local-first software\n- Feedback owed on a friend's draft\n";
