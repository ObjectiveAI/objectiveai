//! Best-effort auto-update on startup.
//!
//! When the `updater` feature is enabled, the binary polls GitHub Releases
//! at most once per [`UPDATE_CHECK_INTERVAL`]. If a newer published release
//! is available for this platform + feature variant, the new binary is
//! downloaded, atomically swapped in place of the running binary, and the
//! CLI re-execs itself with the original argv.
//!
//! **Output policy:** the updater is silent on every "I shouldn't run
//! right now" path — unsupported platform, opt-out env var, dev tree,
//! rate-limit, already-current — and silent on the successful re-exec
//! path. The *only* time it writes anything is when something actually
//! goes wrong (network/disk/swap/etc.), in which case a single
//! `objectiveai: auto-update error: <reason>` line goes to stderr and
//! the CLI proceeds on the current binary. The updater must never
//! prevent normal CLI use.
//!
//! When the `updater` feature is off, [`maybe_auto_update`] is a zero-cost
//! no-op — neither `semver` nor any release-fetching code is compiled in.

use std::ffi::OsString;

use crate::Config;

/// Public entrypoint. Called unconditionally from `main.rs`. When the
/// `updater` feature is disabled this is a no-op; when enabled, it may
/// replace the binary + re-exec (in which case this fn never returns).
///
/// `cli_config` is borrowed from `main.rs` so that env-sourced inputs
/// (notably `GITHUB_AUTHORIZATION`) come through the same `Config`
/// struct the rest of the CLI uses — no env-var names need to be
/// defined in this module.
pub async fn maybe_auto_update<I>(args: I, cli_config: &Config)
where
    I: IntoIterator<Item = OsString> + Clone,
{
    #[cfg(feature = "updater")]
    {
        if let Err(e) = imp::run(args, cli_config).await {
            objectiveai_cli_lib::output::Output::<serde_json::Value>::Error(
                objectiveai_cli_lib::output::Error {
                    level: objectiveai_cli_lib::output::Level::Warn,
                    fatal: false,
                    message: format!("auto-update error: {e}"),
                },
            )
            .emit();
        }
    }
    #[cfg(not(feature = "updater"))]
    {
        let _ = (args, cli_config);
    }
}

#[cfg(feature = "updater")]
mod imp {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// How often to poll the release feed.
    const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(2 * 3600);

    /// Timeouts for the two network hops — keep tight so a flaky network
    /// doesn't add perceptible startup latency.
    const METADATA_TIMEOUT: Duration = Duration::from_secs(5);
    const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

    const RELEASES_API: &str =
        "https://api.github.com/repos/ObjectiveAI/objectiveai/releases/latest";

    /// Setting `OBJECTIVEAI_SKIP_UPDATE` disables the updater. We set it
    /// ourselves on the re-exec to prevent an update loop if the new
    /// binary somehow still thinks it's older.
    const SKIP_ENV_VAR: &str = "OBJECTIVEAI_SKIP_UPDATE";

    /// Asset filename that matches THIS build, selected at compile time
    /// from target triple + `viewer` feature. Unsupported platforms
    /// resolve to `None` and the updater is a no-op for them.
    pub(super) const ASSET_NAME: Option<&str> = asset_name();

    const fn asset_name() -> Option<&'static str> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64", feature = "viewer"))]
        {
            Some("objectiveai-linux-x86_64")
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64", not(feature = "viewer")))]
        {
            Some("objectiveai-linux-x86_64-no-viewer")
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "viewer"))]
        {
            Some("objectiveai-linux-aarch64")
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64", not(feature = "viewer")))]
        {
            Some("objectiveai-linux-aarch64-no-viewer")
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64", feature = "viewer"))]
        {
            Some("objectiveai-macos-x86_64")
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64", not(feature = "viewer")))]
        {
            Some("objectiveai-macos-x86_64-no-viewer")
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64", feature = "viewer"))]
        {
            Some("objectiveai-macos-aarch64")
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64", not(feature = "viewer")))]
        {
            Some("objectiveai-macos-aarch64-no-viewer")
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "viewer"))]
        {
            Some("objectiveai-windows-x86_64.exe")
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64", not(feature = "viewer")))]
        {
            Some("objectiveai-windows-x86_64-no-viewer.exe")
        }
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )))]
        {
            None
        }
    }

    /// Real failures only. The "I shouldn't even try to update right
    /// now" cases (unsupported platform, opt-out env var, dev tree,
    /// rate-limit, already-current) are *not* errors and silently
    /// short-circuit `run` with `Ok(())` — see the early returns there.
    /// Anything that lands in this enum is something the user should
    /// know about: network failure, malformed release, missing asset,
    /// failed swap, etc.
    #[derive(Debug, thiserror::Error)]
    pub(super) enum Error {
        #[error("could not locate current binary: {0}")]
        CurrentExe(std::io::Error),
        #[error("write updated.txt: {0}")]
        WriteMarker(std::io::Error),
        #[error("http: {0}")]
        Http(String),
        #[error("github returned status {0}")]
        BadStatus(reqwest::StatusCode),
        #[error("malformed release metadata: {0}")]
        BadMetadata(serde_json::Error),
        #[error("semver parse: {0}")]
        Semver(semver::Error),
        #[error("no asset named {0} in latest release")]
        NoAsset(&'static str),
        #[error("download: {0}")]
        Download(std::io::Error),
        #[error("swap: {0}")]
        Swap(std::io::Error),
        #[error("re-exec: {0}")]
        ReExec(std::io::Error),
    }

    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
        assets: Vec<Asset>,
    }

    #[derive(serde::Deserialize)]
    struct Asset {
        name: String,
        browser_download_url: String,
    }

    pub(super) async fn run<I>(args: I, cli_config: &super::Config) -> Result<(), Error>
    where
        I: IntoIterator<Item = OsString> + Clone,
    {
        // Silent short-circuit: nothing to do on platforms with no
        // release asset.
        let Some(asset_name) = ASSET_NAME else {
            return Ok(());
        };
        // Silent short-circuit: opt-out via env var (also set by the
        // re-exec to prevent loops).
        if std::env::var_os(SKIP_ENV_VAR).is_some() {
            return Ok(());
        }

        let current_exe = std::env::current_exe().map_err(Error::CurrentExe)?;
        // Silent short-circuit: running out of a `target/` dir is a dev
        // build, never an installed binary.
        if looks_like_dev_tree(&current_exe) {
            return Ok(());
        }

        // Best-effort cleanup of any stale `.exe.old` from a prior Windows
        // swap. If we're not on Windows, or there's no stale file, this
        // does nothing.
        sweep_stale_old(&current_exe);

        // Silent short-circuit: rate-limit gate. The marker lives in the
        // config base dir so it shares the CONFIG_BASE_DIR override used
        // everywhere else.
        let marker = marker_path()?;
        if !check_elapsed(&marker) {
            return Ok(());
        }
        // Refresh the marker BEFORE the network call so a network failure
        // still rate-limits subsequent runs (we won't hammer GitHub).
        write_marker(&marker)?;

        let client = reqwest::Client::new();
        let auth = github_authorization(cli_config).await;
        let release: Release = {
            let mut req = client
                .get(RELEASES_API)
                .header("User-Agent", user_agent())
                .header("Accept", "application/vnd.github+json")
                .timeout(METADATA_TIMEOUT);
            if let Some(header) = auth.as_deref() {
                req = req.header("Authorization", header);
            }
            let resp = req
                .send()
                .await
                .map_err(|e| Error::Http(e.to_string()))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(Error::BadStatus(status));
            }
            let body = resp
                .bytes()
                .await
                .map_err(|e| Error::Http(e.to_string()))?;
            serde_json::from_slice(&body).map_err(Error::BadMetadata)?
        };

        // Compare versions. Bail quietly when we're already current or
        // somehow ahead (pre-release, local dev bump, etc.).
        let remote_str = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
        let remote = semver::Version::parse(remote_str).map_err(Error::Semver)?;
        let local = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(Error::Semver)?;
        if remote <= local {
            return Ok(());
        }

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or(Error::NoAsset(asset_name))?;

        // Download next to the current binary so the eventual rename is a
        // same-filesystem operation (rename across devices fails).
        let new_path = staged_path(&current_exe);
        download_to(&client, &asset.browser_download_url, auth.as_deref(), &new_path).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&new_path, std::fs::Permissions::from_mode(0o755))
                .map_err(Error::Swap)?;
        }

        self_replace(&current_exe, &new_path)?;
        re_exec(&current_exe, args)
    }

    /// Extension used for the downloaded-but-not-yet-installed binary.
    /// Lives next to the target path so the final rename doesn't cross
    /// filesystems.
    fn staged_path(current_exe: &Path) -> PathBuf {
        let mut p = current_exe.to_path_buf();
        let filename = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "objectiveai".to_string());
        let pid = std::process::id();
        p.set_file_name(format!("{filename}.new.{pid}"));
        p
    }

    /// Detect common in-repo paths to avoid clobbering a developer's
    /// `cargo run` output. The release binary lives under
    /// `~/.objectiveai/` (or a user-chosen install path); the dev binary
    /// lives under `<repo>/target/`.
    fn looks_like_dev_tree(current_exe: &Path) -> bool {
        current_exe
            .components()
            .any(|c| {
                let s = c.as_os_str();
                s == "target"
                    || s == "target-objectiveai-mcp-filesystem"
                    || s == "target-objectiveai-mcp-proxy"
            })
    }

    fn user_agent() -> String {
        format!("objectiveai-cli/{}", env!("CARGO_PKG_VERSION"))
    }

    /// The marker is `<config_base_dir>/updated.txt`. Reuses
    /// `objectiveai::filesystem::Client` so CONFIG_BASE_DIR / ~/.objectiveai
    /// resolution matches the rest of the CLI.
    fn marker_path() -> Result<PathBuf, Error> {
        let fs_client = fs_client();
        Ok(fs_client.base_dir().join("updated.txt"))
    }

    fn fs_client() -> objectiveai::filesystem::Client {
        objectiveai::filesystem::Client::new(
            None::<String>,
            None::<String>,
            None::<String>,
        )
    }

    /// Resolves the GitHub Authorization header value, if one is
    /// available. Lookup order (same precedence the rest of the CLI
    /// uses): the env-sourced value on `cli_config` first, then the
    /// filesystem config's stored `api.headers.x_github_authorization`.
    ///
    /// Returns a value already formatted for the `Authorization`
    /// header (`Bearer <token>`). The stored token may or may not
    /// carry the `Bearer ` prefix; both forms are handled.
    async fn github_authorization(cli_config: &super::Config) -> Option<String> {
        let raw = match cli_config
            .github_authorization
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(s) => Some(s.to_string()),
            None => {
                let client = fs_client();
                // Best-effort: if the config file doesn't exist / is
                // malformed, just skip.
                match objectiveai::filesystem::config::client::read(&client).await {
                    Ok(mut config) => config
                        .api()
                        .headers()
                        .get_x_github_authorization()
                        .map(|s| s.to_string()),
                    Err(_) => None,
                }
            }
        };
        raw.map(|s| {
            // The stored form may or may not include the scheme
            // prefix. Strip it (if present) and re-prepend, so the
            // outgoing header is always `Bearer <token>` exactly once.
            let trimmed = s.trim();
            let raw = trimmed.strip_prefix("Bearer ").unwrap_or(trimmed);
            format!("Bearer {raw}")
        })
    }

    /// Returns true iff enough time has elapsed since the marker was last
    /// updated. A missing or unreadable marker counts as "elapsed".
    fn check_elapsed(marker: &Path) -> bool {
        let Ok(contents) = std::fs::read_to_string(marker) else {
            return true;
        };
        let Ok(ts) = contents.trim().parse::<u64>() else {
            return true;
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(ts) >= UPDATE_CHECK_INTERVAL.as_secs()
    }

    fn write_marker(marker: &Path) -> Result<(), Error> {
        if let Some(parent) = marker.parent() {
            std::fs::create_dir_all(parent).map_err(Error::WriteMarker)?;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        std::fs::write(marker, now.to_string()).map_err(Error::WriteMarker)
    }

    async fn download_to(
        client: &reqwest::Client,
        url: &str,
        auth: Option<&str>,
        dst: &Path,
    ) -> Result<(), Error> {
        use futures::StreamExt as _;
        use tokio::io::AsyncWriteExt as _;

        let mut req = client
            .get(url)
            .header("User-Agent", user_agent())
            .timeout(DOWNLOAD_TIMEOUT);
        if let Some(header) = auth {
            req = req.header("Authorization", header);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::BadStatus(status));
        }

        let mut file = tokio::fs::File::create(dst).await.map_err(Error::Download)?;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| Error::Http(e.to_string()))?;
            file.write_all(&chunk).await.map_err(Error::Download)?;
        }
        file.flush().await.map_err(Error::Download)?;
        Ok(())
    }

    /// Swap the staged binary into place of the currently-running one.
    ///
    /// **Unix**: `rename(new, current)` works because the running process
    /// holds the binary by inode; overwriting the path doesn't unload the
    /// live image, the old inode stays alive until the process exits.
    ///
    /// **Windows**: the running exe's path is locked for *writes*, but
    /// *renaming* the running file to a different name is allowed (the
    /// lock is on the path, not the inode). So we move the current binary
    /// aside first, then drop the new one into the original path.
    #[cfg(unix)]
    fn self_replace(current: &Path, new: &Path) -> Result<(), Error> {
        std::fs::rename(new, current).map_err(Error::Swap)
    }

    #[cfg(windows)]
    fn self_replace(current: &Path, new: &Path) -> Result<(), Error> {
        let old = current.with_extension("exe.old");
        // Clear any previous `.old` that failed to get swept.
        let _ = std::fs::remove_file(&old);
        std::fs::rename(current, &old).map_err(Error::Swap)?;
        std::fs::rename(new, current).map_err(|e| {
            // Best effort: try to undo the rename so the user doesn't end
            // up with a missing binary on the PATH.
            let _ = std::fs::rename(&old, current);
            Error::Swap(e)
        })
    }

    #[cfg(not(any(unix, windows)))]
    fn self_replace(_current: &Path, _new: &Path) -> Result<(), Error> {
        Err(Error::Swap(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "self-replace not implemented on this platform",
        )))
    }

    fn sweep_stale_old(current: &Path) {
        #[cfg(windows)]
        {
            let old = current.with_extension("exe.old");
            let _ = std::fs::remove_file(old);
        }
        #[cfg(not(windows))]
        {
            let _ = current;
        }
    }

    /// Re-exec the (now-updated) binary with the same argv. On all
    /// platforms we spawn + wait rather than use `exec(2)` — the tokio
    /// runtime is up, and using `exec` from inside an async fn would
    /// leak the runtime. The extra process layer is free at user-scale.
    ///
    /// This is strictly a passthrough:
    /// - **stdin / stdout / stderr** are inherited from the parent, so
    ///   piped input and terminal output behave identically to running
    ///   the new binary directly.
    /// - **Environment** is inherited unchanged except for `OBJECTIVEAI_SKIP_UPDATE=1`,
    ///   which stops the child from re-running the updater and looping.
    /// - **Current directory** is inherited (Command's default).
    /// - **Argv** forwards every user-supplied argument verbatim;
    ///   argv[0] is set to the binary path (on Unix we preserve the
    ///   original argv[0] via `CommandExt::arg0` so tools that read
    ///   their own invocation name see the same string they were given).
    /// - **Exit code** mirrors the child's. Signal-terminated children
    ///   propagate as `128 + signum` on Unix (POSIX convention); on
    ///   Windows a child without an exit code maps to `1`.
    fn re_exec<I>(current: &Path, args: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = OsString>,
    {
        use std::process::{Command, Stdio};

        let mut iter = args.into_iter();
        let argv0 = iter.next(); // original invocation name
        let forwarded: Vec<OsString> = iter.collect();

        let mut cmd = Command::new(current);
        cmd.args(&forwarded)
            // Make inheritance explicit so future changes can't
            // accidentally capture or close a handle.
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            // Break the update loop: the re-exec'd child must not
            // re-enter the updater even if its version check would
            // somehow trigger again.
            .env(SKIP_ENV_VAR, "1");

        // On Unix, preserve argv[0] exactly so the child sees its
        // original invocation name (matters for clap's program name in
        // help output, shell completions, etc.). Windows' CreateProcess
        // derives argv[0] from the command string and doesn't let us
        // override it separately, so there it becomes the exe path.
        #[cfg(unix)]
        if let Some(argv0) = argv0.as_ref() {
            use std::os::unix::process::CommandExt as _;
            cmd.arg0(argv0);
        }
        #[cfg(not(unix))]
        let _ = argv0;

        let status = cmd.status().map_err(Error::ReExec)?;

        // Propagate exit code faithfully. `None` here means "terminated
        // by a signal" on Unix; fall back to `128 + signum`. On Windows
        // `None` is the rare "no exit code available" case — use 1.
        let code = match status.code() {
            Some(c) => c,
            None => {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt as _;
                    status.signal().map(|s| 128 + s).unwrap_or(1)
                }
                #[cfg(not(unix))]
                {
                    1
                }
            }
        };
        std::process::exit(code);
    }
}

// ---------------------------------------------------------------------------
// Tests — cover only the feature-on configuration so they don't duplicate
// for each feature matrix; they're compiled in alongside the real impl.
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "updater"))]
mod tests {
    #[test]
    fn asset_name_resolves_for_current_target() {
        // On every supported CI target, ASSET_NAME is Some(...). The cfg
        // matrix in asset_name() would otherwise silently slip into None.
        #[cfg(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        ))]
        assert!(super::imp::ASSET_NAME.is_some());
    }

    #[test]
    fn version_ordering() {
        fn needs_update(remote: &str, local: &str) -> bool {
            let r = remote.strip_prefix('v').unwrap_or(remote);
            semver::Version::parse(r).unwrap() > semver::Version::parse(local).unwrap()
        }
        assert!(needs_update("v2.0.1", "2.0.0"));
        assert!(needs_update("2.0.1", "2.0.0"));
        assert!(!needs_update("v2.0.0", "2.0.0"));
        assert!(!needs_update("v2.0.0", "2.1.0"));
        assert!(needs_update("v3.0.0", "2.99.99"));
    }
}
