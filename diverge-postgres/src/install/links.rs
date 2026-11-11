//! The archive's hard links, made at every extraction.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use flate2::bufread::GzDecoder;
use postgresql_archive::configuration::theseus;
use postgresql_archive::extractor::{ExtractDirectories, registry};
use tar::Archive;

/// Have every extraction of the baked-in archive make its hard links
/// too: this file's extractor registered as the extractor of the
/// archive's repository, ahead of the one `postgresql_archive`
/// registers for it — the last registered is the one asked. The
/// rules:
///
/// 1. **Why.** The archive names some of its files twice, the second
///    name a tar hard link to the first: every time zone that is
///    another's alias, `UTC` among them. `postgresql_archive` 0.20
///    extracts the directories, the files and the symbolic links, and
///    passes over the rest; a server without `share/timezone/UTC`
///    refuses a connection that asks for `TimeZone=UTC`, which is
///    every connection the daemon makes. `postgresql_archive` 0.22
///    makes the links itself, and this goes when the dependency
///    reaches it.
/// 2. **Here, and not after.** The archive's bytes are
///    `postgresql_embedded`'s own, handed to nothing but the extractor
///    it asks the registry for: that is the one place they can be read
///    a second time.
/// 3. **Once is enough.** A second registration is the same extractor
///    ahead of itself, and changes nothing.
pub fn links() -> postgresql_archive::Result<()> {
    registry::register(|url| Ok(url.starts_with(theseus::URL)), extract)
}

/// One extraction of the archive `bytes` are, links and all: what
/// `postgresql_archive` extracts of it, into the directory
/// `directories` name, and then every hard link it names, [`make`]d
/// there. Its errors are the crate's own, `Unexpected` for what the
/// archive names wrongly: the registry takes a function of this
/// shape, and no other.
fn extract(bytes: &Vec<u8>, directories: &ExtractDirectories) -> postgresql_archive::Result<Vec<PathBuf>> {
    let mut files = theseus::extract(bytes, directories)?;
    let home = directories.get_path(".")?;
    files.extend(make(bytes, &home)?);
    Ok(files)
}

/// Every hard link the archive `bytes` are names, made under `home`,
/// where the archive was extracted. The rules:
///
/// 1. **Read from the archive.** Its entries are read again, and each
///    of type link gives a name and the name of its target, both
///    paths in the archive — see [`under`].
/// 2. **Copied.** A link is made as a copy of its target: a hard link
///    is not to be had on every filesystem, and a copy is the same
///    file to the server. One already there is left as it is.
/// 3. **Until none waits.** A link whose target is not there waits —
///    the target may be a link not made yet — and passes are made
///    until one makes nothing. The links still waiting then name
///    targets the archive does not hold, which is the error.
fn make(bytes: &[u8], home: &Path) -> postgresql_archive::Result<Vec<PathBuf>> {
    // Rule 1.
    let mut archive = Archive::new(GzDecoder::new(bytes));
    let mut waiting = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        if !entry.header().entry_type().is_hard_link() {
            continue;
        }
        let link = under(home, &entry.path()?)?;
        let Some(target) = entry.link_name()? else {
            return Err(postgresql_archive::Error::Unexpected(format!("the archive's hard link `{}` names no target", link.display())));
        };
        waiting.push((link, under(home, &target)?));
    }
    let mut made = Vec::new();
    // Rule 3.
    loop {
        let mut still = Vec::new();
        let before = waiting.len();
        for (link, target) in waiting {
            if link.is_file() {
                continue;
            }
            if !target.is_file() {
                still.push((link, target));
                continue;
            }
            // Rule 2.
            if let Some(parent) = link.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&target, &link)?;
            made.push(link);
        }
        let Some((_, target)) = still.first() else {
            return Ok(made);
        };
        if still.len() == before {
            return Err(postgresql_archive::Error::Unexpected(format!("{} of the archive's hard links name targets it does not hold, `{}` the first", still.len(), target.display())));
        }
        waiting = still;
    }
}

/// The archive's path `path` as a path under `home`: every component
/// a [`plain`] name, and the first of them — the archive's one top
/// directory — left out, as the extraction leaves it out. Any other
/// component — a `.`, a `..`, a root, a prefix — is no path inside
/// the archive, and is refused, wherever it stands.
fn under(home: &Path, path: &Path) -> postgresql_archive::Result<PathBuf> {
    let mut found = home.to_path_buf();
    for (index, component) in path.components().enumerate() {
        match component {
            Component::Normal(name) if plain(name) => {
                if index > 0 {
                    found.push(name);
                }
            }
            _ => return Err(postgresql_archive::Error::Unexpected(format!("the archive names `{}`, which is no path inside it", path.display()))),
        }
    }
    Ok(found)
}

/// Whether `name`, one component of an archive's path, is one plain
/// name on this host too. Within a path Windows reads `C:` as a name,
/// and alone as a drive: pushed onto a path it is not joined to it,
/// and replaces it.
fn plain(name: &OsStr) -> bool {
    let mut read = Path::new(name).components();
    matches!((read.next(), read.next()), (Some(Component::Normal(_)), None))
}
