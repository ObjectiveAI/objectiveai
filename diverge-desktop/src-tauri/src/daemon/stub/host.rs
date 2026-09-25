//! The stand-in daemon's host: a real folder on this Mac that plays the
//! part of the machine the daemon runs on. `filesystem::*` reads and writes
//! real files in it; nothing outside it is ever touched.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use diverge_provider_sdk::shared::filetree::response::{Frame as TreeFrame, Node};

pub struct Host {
    root: PathBuf,
}

impl Host {
    pub fn new(root: PathBuf) -> Self {
        let host = Host { root };
        host.seed();
        host
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A daemon path — absolute, as the host writes one — to a real path
    /// under the stand-in's folder. `..` never leaves it.
    pub fn resolve(&self, daemon_path: &str) -> Result<PathBuf, String> {
        if !daemon_path.starts_with('/') {
            return Err(format!("\"{daemon_path}\" is not an absolute path"));
        }
        let mut out = self.root.clone();
        for component in Path::new(daemon_path).components() {
            match component {
                Component::RootDir | Component::CurDir => {}
                Component::Normal(part) => out.push(part),
                Component::ParentDir | Component::Prefix(_) => {
                    return Err(format!("\"{daemon_path}\" leaves the host"));
                }
            }
        }
        Ok(out)
    }

    pub fn exists(&self, daemon_path: &str) -> bool {
        self.resolve(daemon_path).map(|p| p.exists()).unwrap_or(false)
    }

    pub fn read(&self, daemon_path: &str) -> Result<Vec<u8>, String> {
        let path = self.resolve(daemon_path)?;
        if path.is_dir() {
            return Err(format!("{daemon_path} is a directory"));
        }
        fs::read(&path).map_err(|e| format!("{daemon_path}: {e}"))
    }

    pub fn read_text(&self, daemon_path: &str) -> String {
        self.read(daemon_path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
    }

    /// Put a file in place whole: written beside, then renamed over.
    pub fn write(&self, daemon_path: &str, body: &[u8]) -> Result<(), String> {
        let path = self.resolve(daemon_path)?;
        let parent = path.parent().ok_or_else(|| format!("{daemon_path} has no parent"))?;
        if !parent.is_dir() {
            return Err(format!("the folder for {daemon_path} does not exist"));
        }
        if path.is_dir() {
            return Err(format!("{daemon_path} is a directory"));
        }
        let temp = parent.join(format!(".{}.writing", path.file_name().unwrap_or_default().to_string_lossy()));
        fs::write(&temp, body).map_err(|e| format!("{daemon_path}: {e}"))?;
        fs::rename(&temp, &path).map_err(|e| format!("{daemon_path}: {e}"))
    }

    /// The entries under a directory, recursively, as the wire names them.
    pub fn tree(&self, daemon_path: &str) -> Result<Vec<Node>, String> {
        let path = self.resolve(daemon_path)?;
        if !path.is_dir() {
            return Err(format!("{daemon_path} is not a directory"));
        }
        Ok(entries(&path, &path))
    }

    fn seed(&self) {
        let files: &[(&str, &str)] = &[
            ("projects/site/index.html", SITE_INDEX),
            ("projects/site/styles.css", SITE_CSS),
            ("projects/site/README.md", SITE_README),
            ("notes/ideas.md", NOTES_IDEAS),
            ("notes/reading-list.md", NOTES_READING),
            ("datasets/README.md", "# datasets\n\nNothing here yet.\n"),
        ];
        for (rel, body) in files {
            let path = self.root.join(rel);
            if path.exists() {
                continue;
            }
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(path, body);
        }
    }
}

fn secs(time: std::io::Result<std::time::SystemTime>) -> Option<u64> {
    time.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs())
}

fn entries(root: &Path, dir: &Path) -> Vec<Node> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    let mut nodes: Vec<Node> = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".writing") && name.starts_with('.') {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(entry.path()) else { continue };
        let created_at = secs(meta.created());
        let modified_at = secs(meta.modified());
        if meta.file_type().is_symlink() {
            let target = fs::read_link(entry.path()).unwrap_or_default();
            let path = target
                .strip_prefix(root)
                .unwrap_or(&target)
                .components()
                .filter_map(|c| match c {
                    Component::Normal(p) => Some(p.to_string_lossy().into_owned()),
                    _ => None,
                })
                .collect();
            nodes.push(Node::Symlink { name, path, created_at, modified_at });
        } else if meta.is_dir() {
            let children = entries(root, &entry.path());
            nodes.push(Node::Directory { name, created_at, modified_at, changes: true, children });
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

/// The deltas that take `old` to `new`: one frame per node that appeared,
/// changed in place, or ceased to exist. Paths are relative to the root.
pub fn diff(prefix: &[String], old: &[Node], new: &[Node], out: &mut Vec<TreeFrame>) {
    let old_by: BTreeMap<&str, &Node> = old.iter().map(|n| (name_of(n), n)).collect();
    let new_by: BTreeMap<&str, &Node> = new.iter().map(|n| (name_of(n), n)).collect();
    for (name, node) in &new_by {
        let mut path = prefix.to_vec();
        path.push((*name).to_owned());
        match old_by.get(name) {
            None => out.push(TreeFrame::Inserted { path, node: (*node).clone() }),
            Some(before) => match (before, node) {
                (Node::Directory { children: a, .. }, Node::Directory { children: b, .. }) => diff(&path, a, b, out),
                (before, after) if std::mem::discriminant(*before) != std::mem::discriminant(*after) => {
                    out.push(TreeFrame::Removed { path: path.clone() });
                    out.push(TreeFrame::Inserted { path, node: (*after).clone() });
                }
                (before, after) if before != after => out.push(TreeFrame::Modified { path, node: (*after).clone() }),
                _ => {}
            },
        }
    }
    for name in old_by.keys() {
        if !new_by.contains_key(name) {
            let mut path = prefix.to_vec();
            path.push((*name).to_owned());
            out.push(TreeFrame::Removed { path });
        }
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

const NOTES_IDEAS: &str = "# ideas\n\n- A shop on my profile for the poster packs\n- Ask if the studio PC's GPU can be borrowed on weekends\n- What does a bounty look like before money exists?\n- Onboarding without a tour\n";

const NOTES_READING: &str = "# reading list\n\n- The provider protocol, 2.3.0\n- Notes on local-first software\n- Feedback owed on a friend's draft\n";
