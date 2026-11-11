//! Whether an extraction has the archive's hard links.

use std::io::ErrorKind;
use std::path::Path;

/// The time zone every connection of the daemon's asks the server
/// for, under an extraction's `share/timezone/`: one of the archive's
/// hard links.
const ZONE: &str = "UTC";

/// Whether the extraction at `home` has the links its archive named,
/// as far as can be told with no archive to read — by the one of them
/// the daemon cannot do without: an extraction that carries its own
/// time zones, a `share/timezone/`, has `UTC` among them, a file. One
/// with no `share/timezone/` takes its zones from the system, and has
/// none to be without. Only a `UTC` that is not there, or is no file,
/// says unlinked: one that cannot be asked after says nothing, and no
/// extraction is thrown away on that.
pub async fn linked(home: &Path) -> bool {
    let zones = home.join("share").join("timezone");
    if !tokio::fs::try_exists(&zones).await.unwrap_or(false) {
        return true;
    }
    match tokio::fs::metadata(zones.join(ZONE)).await {
        Ok(found) => found.is_file(),
        Err(error) => error.kind() != ErrorKind::NotFound,
    }
}
