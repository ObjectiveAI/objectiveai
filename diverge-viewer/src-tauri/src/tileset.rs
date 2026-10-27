//! The world's tilesets: each a folder under `world/tilesets/`, holding
//! its manifest, `tileset.json`, and the atlas image the manifest names.
//!
//! The world folder is `<root>/viewer/world/`, copied there from the
//! repo's `diverge-viewer/world/`; the app reads it and never writes it,
//! and has no copy of its own. A later tileset is a new folder beside the
//! others, and a later kind of world data a new folder beside
//! `tilesets/`.
//!
//! The rules a tileset is held to, in order:
//!
//! 1. Its id is its folder's name: one path component, with no
//!    separator, and neither `.` nor `..`.
//! 2. Its manifest is `tileset.json` in that folder. It names every one
//!    of the six tile kinds — water, land, account, agent, standalone
//!    tool, dependency tool — and no other key, here or anywhere in it.
//! 3. A glyph is a cell index into the atlas, counted row by row from
//!    the top left, and is below `columns × rows`. The atlas is in code
//!    page 437's order, so a Dwarf Fortress tileset's image drops in and
//!    its glyph numbers carry over.
//! 4. The image is a bare file name in the tileset's own folder, as an
//!    id is (rule 1). It is a PNG exactly `columns × cell.width` pixels
//!    wide and `rows × cell.height` high.
//! 5. Every colour is `#rrggbb`.
//!
//! How a cell is drawn — `bg`, then the glyph multiplied by `fg`, with
//! its transparent and `#FF00FF` pixels showing `bg` — is the page's
//! (`src/world/tileset.ts`).

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The world folder, in the app's own folder.
pub const WORLD: &str = "world";

/// The folder of every tileset, in the world folder.
const TILESETS: &str = "tilesets";

/// A tileset's manifest, in its folder.
const MANIFEST: &str = "tileset.json";

/// The eight bytes every PNG begins with.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// A tileset's manifest: its name, its atlas, and the tile of every kind.
#[derive(Deserialize, Serialize, TS, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct Tileset {
    /// The name it is shown under.
    pub name: String,
    /// The image the glyphs are cut from.
    pub atlas: Atlas,
    /// The tile of every kind.
    pub tiles: Tiles,
}

/// An atlas: one image cut into a grid of equal cells, one glyph each.
#[derive(Deserialize, Serialize, TS, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct Atlas {
    /// The image's file name, in the tileset's folder.
    pub image: String,
    /// How many cells across.
    pub columns: u32,
    /// How many cells down.
    pub rows: u32,
    /// One cell's size, in pixels.
    pub cell: Cell,
}

/// One cell's size, in pixels.
#[derive(Deserialize, Serialize, TS, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct Cell {
    pub width: u32,
    pub height: u32,
}

/// The tile of every kind the world draws.
#[derive(Deserialize, Serialize, TS, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct Tiles {
    pub water: Tile,
    pub land: Tile,
    pub account: Tile,
    pub agent: Tile,
    pub standalone_tool: Tile,
    pub dependency_tool: Tile,
}

impl Tiles {
    /// Every tile, by its kind's name.
    fn each(&self) -> [(&'static str, &Tile); 6] {
        [
            ("water", &self.water),
            ("land", &self.land),
            ("account", &self.account),
            ("agent", &self.agent),
            ("standalone_tool", &self.standalone_tool),
            ("dependency_tool", &self.dependency_tool),
        ]
    }
}

/// One tile: a glyph of the atlas, and the two colours it is drawn in.
#[derive(Deserialize, Serialize, TS, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct Tile {
    /// The glyph's cell index in the atlas, row by row from the top left.
    pub glyph: u32,
    /// The colour the glyph is multiplied by, `#rrggbb`.
    pub fg: String,
    /// The colour under the glyph, `#rrggbb`.
    pub bg: String,
}

/// One tileset as the list shows it: its id, and its name or why it
/// cannot be used.
#[derive(Serialize, TS, Clone, Debug)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct TilesetEntry {
    /// Its folder's name.
    pub id: String,
    /// Its name, when it holds up.
    pub name: Option<String>,
    /// Why it cannot be used, when it does not.
    pub error: Option<String>,
}

/// Why a tileset cannot be used.
#[derive(Debug)]
pub enum Error {
    /// An id or an image that is not one bare file name.
    NotBare(String),
    /// A file or folder that could not be read.
    Read { path: PathBuf, source: std::io::Error },
    /// A manifest that is not one.
    Parse { path: PathBuf, source: serde_json::Error },
    /// A glyph past the atlas's last cell.
    Glyph { kind: &'static str, glyph: u32, cells: u32 },
    /// A colour that is not `#rrggbb`.
    Colour { kind: &'static str, value: String },
    /// An image that is not a PNG.
    NotPng(PathBuf),
    /// An image of another size than the atlas says.
    Size { path: PathBuf, expected: (u32, u32), found: (u32, u32) },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotBare(name) => write!(f, "{name:?} is not one bare file name"),
            Error::Read { path, source } => write!(f, "{} could not be read: {source}", path.display()),
            Error::Parse { path, source } => write!(f, "{} is not a tileset: {source}", path.display()),
            Error::Glyph { kind, glyph, cells } => write!(f, "the {kind} tile's glyph {glyph} is past the atlas's {cells} cells"),
            Error::Colour { kind, value } => write!(f, "the {kind} tile's colour {value:?} is not #rrggbb"),
            Error::NotPng(path) => write!(f, "{} is not a PNG", path.display()),
            Error::Size { path, expected, found } => write!(
                f,
                "{} is {}×{} pixels, and the atlas says {}×{}",
                path.display(),
                found.0,
                found.1,
                expected.0,
                expected.1
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Whether `name` is one bare file name: rules 1 and 4.
fn bare(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':'])
}

/// Whether `value` is `#rrggbb`: rule 5.
fn colour(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// The folder of the tileset `id`, under the world folder `world`.
fn folder(world: &Path, id: &str) -> Result<PathBuf, Error> {
    if !bare(id) {
        return Err(Error::NotBare(id.to_owned()));
    }
    Ok(world.join(TILESETS).join(id))
}

/// The tileset `id`, held to every rule.
pub async fn load(world: &Path, id: &str) -> Result<Tileset, Error> {
    let folder = folder(world, id)?;
    let path = folder.join(MANIFEST);
    let bytes = tokio::fs::read(&path).await.map_err(|source| Error::Read { path: path.clone(), source })?;
    let tileset: Tileset = serde_json::from_slice(&bytes).map_err(|source| Error::Parse { path, source })?;
    let cells = tileset.atlas.columns * tileset.atlas.rows;
    for (kind, tile) in tileset.tiles.each() {
        if tile.glyph >= cells {
            return Err(Error::Glyph { kind, glyph: tile.glyph, cells });
        }
        for value in [&tile.fg, &tile.bg] {
            if !colour(value) {
                return Err(Error::Colour { kind, value: value.clone() });
            }
        }
    }
    let image = read_image(&folder, &tileset.atlas.image).await?;
    let expected = (tileset.atlas.columns * tileset.atlas.cell.width, tileset.atlas.rows * tileset.atlas.cell.height);
    let found = png_size(&image).ok_or_else(|| Error::NotPng(folder.join(&tileset.atlas.image)))?;
    if found != expected {
        return Err(Error::Size { path: folder.join(&tileset.atlas.image), expected, found });
    }
    Ok(tileset)
}

/// The atlas image of the tileset `id`, as it is held to every rule.
pub async fn image(world: &Path, id: &str) -> Result<Vec<u8>, Error> {
    let tileset = load(world, id).await?;
    read_image(&folder(world, id)?, &tileset.atlas.image).await
}

/// Every tileset in the world folder, by id: each with its name, or why
/// it cannot be used. A world folder that cannot be read is the error.
pub async fn list(world: &Path) -> Result<Vec<TilesetEntry>, Error> {
    let path = world.join(TILESETS);
    let mut read = tokio::fs::read_dir(&path).await.map_err(|source| Error::Read { path: path.clone(), source })?;
    let mut ids = Vec::new();
    while let Some(entry) = read.next_entry().await.map_err(|source| Error::Read { path: path.clone(), source })? {
        let is_dir = entry.file_type().await.is_ok_and(|kind| kind.is_dir());
        if is_dir {
            ids.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    ids.sort();
    let mut entries = Vec::with_capacity(ids.len());
    for id in ids {
        entries.push(match load(world, &id).await {
            Ok(tileset) => TilesetEntry { id, name: Some(tileset.name), error: None },
            Err(error) => TilesetEntry { id, name: None, error: Some(error.to_string()) },
        });
    }
    Ok(entries)
}

/// The image `name`, in the tileset's `folder`: rule 4's name, read.
async fn read_image(folder: &Path, name: &str) -> Result<Vec<u8>, Error> {
    if !bare(name) {
        return Err(Error::NotBare(name.to_owned()));
    }
    let path = folder.join(name);
    tokio::fs::read(&path).await.map_err(|source| Error::Read { path, source })
}

/// A PNG's width and height, from its header: the signature, then the
/// `IHDR` chunk, whose data begins with the two as big-endian `u32`s.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || bytes[..8] != PNG_SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}
