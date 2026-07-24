//! The viewer-extension INSTALLER: turn a plugin repo's declared
//! `viewer` root into installed, self-contained tab bundles under
//! `<OBJECTIVEAI_DIR>/bin/plugins/<owner>/<name>/<version>/`.
//!
//! The pipeline mirrors the laboratory host's container-image build
//! (`plugin_image::ensure` over there): machine-wide file lock →
//! double-checked probe → single-tag git fetch (local-override-first)
//! into this side's OWN temp partition (`<bin>/temp/viewer` — the
//! host owns the sibling `<bin>/temp/daemon`) → work → cleanup on
//! every path. Everything validates IN TEMP and fails loudly there;
//! nothing lands in bin/plugins unless the whole pipeline succeeds.
//!
//! The commands here (`plugins_list` / `plugins_install` /
//! `plugins_uninstall`) are the plugins tab's surface — root-identity
//! callers only.

use std::path::PathBuf;

/// The installer's directory layout, derived from `OBJECTIVEAI_DIR`
/// once — managed state so every command shares one source of truth.
pub struct PluginsDirs {
    objectiveai_dir: PathBuf,
}

impl PluginsDirs {
    pub fn new(objectiveai_dir: PathBuf) -> Self {
        Self { objectiveai_dir }
    }

    /// `<dir>/bin/plugins` — the installed-plugin tree (machine-wide,
    /// shared across states).
    pub fn plugins_root(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("plugins")
    }

    /// `<dir>/bin/temp/viewer` — the viewer's OWN temp partition
    /// (checkouts + staging; the laboratory host owns the sibling
    /// `daemon` partition).
    pub fn temp_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("temp").join("viewer")
    }

    /// `<dir>/bin/locks` — the machine-wide lock dir the laboratory
    /// host also uses (distinct keys: `plugin-viewer-*` here,
    /// `plugin-image-*` there).
    fn locks_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("bin").join("locks")
    }

    /// `<dir>/plugins` — the local plugin-repo override root the
    /// fetch consults before GitHub.
    fn override_dir(&self) -> PathBuf {
        self.objectiveai_dir.join("plugins")
    }
}

/// Every installed plugin VERSION, for the plugins tab's list — the
/// full tree, not `scan`'s highest-per-plugin reduction.
#[tauri::command]
pub async fn plugins_list(
    app: tauri::AppHandle,
    webview: tauri::Webview,
    model: tauri::State<'_, super::ShellModel>,
    dirs: tauri::State<'_, PluginsDirs>,
) -> Result<Vec<super::PluginVersionInfo>, String> {
    if super::sender_identity(&webview, &model).await != super::ROOT_IDENTITY {
        return Err("plugins_list: root identity only".to_string());
    }
    Ok(super::list_all_versions(&app, &dirs.plugins_root()).await)
}
