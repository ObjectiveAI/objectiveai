//! A name, as a folder's or a file's.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

/// Every byte but a letter, a digit, `-`, `.`, `_` and `~` is
/// encoded: URL's unreserved characters, each of which every
/// filesystem takes.
const ENCODED: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

/// The names Windows reserves for devices, whatever follows them
/// after a dot and in any case.
const DEVICES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// `name` as a folder or a file may be named on every platform, and
/// read back by [`decode`]:
///
/// 1. Every byte of its UTF-8 but `A–Z a–z 0–9 - . _ ~` is `%XX`, the
///    hex uppercase.
/// 2. A name that would then be `.` or `..`, or begin with a Windows
///    device name — `CON`, `NUL`, `COM1` and the rest, before a dot or
///    alone, in any case — has its first character encoded too.
/// 3. A trailing `.`, which Windows drops from a name, is encoded.
pub(super) fn encode(name: &str) -> String {
    let mut encoded = utf8_percent_encode(name, ENCODED).to_string();
    let stem = encoded.split('.').next().unwrap_or_default();
    let device = DEVICES.iter().any(|device| device.eq_ignore_ascii_case(stem));
    if encoded == "." || encoded == ".." || device {
        let first = encoded.remove(0);
        encoded.insert_str(0, &format!("%{:02X}", u32::from(first)));
    }
    if encoded.ends_with('.') {
        encoded.pop();
        encoded.push_str("%2E");
    }
    encoded
}

/// The name `encoded` was made from, or `None` when it is not one
/// [`encode`] made: escapes that are not UTF-8.
pub(super) fn decode(encoded: &str) -> Option<String> {
    percent_decode_str(encoded).decode_utf8().ok().map(|name| name.into_owned())
}
