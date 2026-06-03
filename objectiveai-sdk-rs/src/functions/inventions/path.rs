//! Path encoding for invention task hierarchies.
//!
//! ── HOW FUNCTION NAMES IDENTIFY DESCENDANTS ─────────────────────────
//!
//! Every invention task gets a `name` of the form `<base-name>-<suffix>`
//! where `<suffix>` is the bijective base-255 encoding of the task's
//! position in the invention tree, rendered as base62. When a child
//! task is being created, `state::child_name` looks at the parent's
//! name, decides whether the trailing `-<segment>` is already a path
//! encoding, and either:
//!
//!   - extends that path in place (real path detected), or
//!   - appends a *new* path segment (no path detected; the segment
//!     belongs to the user's original name, e.g. `-v`, `-vii`,
//!     `-final`, `-prod`).
//!
//! ── THE INVARIANT YOU MUST NOT BREAK ────────────────────────────────
//!
//! A path suffix is **always exactly [`PATH_SUFFIX_LEN`] base62
//! characters**. Detection in `child_name` / `reindex_name` requires
//! BOTH:
//!
//!   1. the trailing `-<segment>` is exactly `PATH_SUFFIX_LEN` chars
//!      long, AND
//!   2. it parses successfully via [`b62_to_path`] (i.e. it is valid
//!      base62 AND decodes to a structurally valid path).
//!
//! Both conditions are required. Length alone isn't enough — a 6-char
//! string with a non-base62 character would falsely qualify. A
//! successful parse alone isn't enough — that's the bug that ate
//! `-v` and `-vii` (both happily decode as base62 integers and then
//! as paths, because every `[1, MAX]` u128 round-trips back into
//! `Vec<u64>`).
//!
//! ── THINGS YOU (FUTURE CLAUDE) MUST NEVER DO ────────────────────────
//!
//!   * DO NOT change `path_to_b62` to pad / fix-width / left-pad with
//!     zeros / append marker chars / use a sigil. The encoder is
//!     stable and produces variable-length raw base62 on purpose.
//!     Existing function names in the wild were written with this
//!     encoder; changing it desynchronizes detection from generation.
//!   * DO NOT switch from base62 to hex / base32 / base64 / decimal /
//!     anything else. The detection check assumes base62 alphabet.
//!   * DO NOT change [`PATH_SUFFIX_LEN`] casually. It is part of the
//!     on-the-wire convention.
//!   * DO NOT add a separate "discriminator" character (like a
//!     leading `_` or doubled `--`) to disambiguate path suffixes.
//!     The length+alphabet check is the discriminator.
//!   * DO NOT relax the detection: removing either the length check
//!     or the parse check brings back the `-v` / `-vii` bug.
//!
//! If a future requirement seems to demand changing any of the above,
//! stop and write up the full implication chain (existing names,
//! external readers, JSON schemas, agent prompts, mock data) before
//! touching this file.

use std::fmt::Display;

pub trait PathElement:
    Copy + Into<u128> + TryFrom<u128> + Ord + Display
{
}

macro_rules! impl_path_element {
    ($($t:ty),*) => {
        $(impl PathElement for $t {})*
    };
}

impl_path_element!(u8, u16, u32, u64, u128);

const MAX_LEN: usize = 16;
const MAX_VAL: u128 = 254;

/// The on-the-wire length, in base62 characters, of a path suffix in a
/// function name. Detection (in `state::child_name` /
/// `state::reindex_name`) requires the trailing `-<segment>` to be
/// **exactly this many** characters before even attempting to parse it
/// as a path. See the module docs for the full invariant.
pub const PATH_SUFFIX_LEN: usize = 6;

fn validate_path<T: PathElement>(path: &[T]) -> Result<(), String> {
    if path.len() > MAX_LEN {
        return Err(format!(
            "path length {} exceeds maximum of {MAX_LEN}",
            path.len()
        ));
    }
    for (i, &v) in path.iter().enumerate() {
        if v.into() > MAX_VAL {
            return Err(format!(
                "path[{i}] value {v} exceeds maximum of {MAX_VAL}"
            ));
        }
    }
    Ok(())
}

/// Bijective base-255 encoding: each value v is stored as v+1 (digits 1-255).
/// Since there is no zero digit, different-length paths always produce
/// different u128 values (e.g. [] → 0, [0] → 1, [0,0] → 256).
pub fn path_to_u128<T: PathElement>(path: &[T]) -> Result<u128, String> {
    validate_path(path)?;
    let mut result: u128 = 0;
    for &v in path {
        result = result * 255 + v.into() + 1;
    }
    Ok(result)
}

pub fn u128_to_path<T: PathElement>(
    mut encoded: u128,
) -> Result<Vec<T>, String> {
    let mut path = Vec::new();
    while encoded > 0 {
        encoded -= 1;
        let digit = encoded % 255;
        path.push(T::try_from(digit).map_err(|_| {
            format!("value {digit} out of range for target type")
        })?);
        encoded /= 255;
    }
    if path.len() > MAX_LEN {
        return Err(format!(
            "decoded length {} exceeds maximum of {MAX_LEN}",
            path.len()
        ));
    }
    path.reverse();
    Ok(path)
}

pub fn u128_to_b62(v: u128) -> String {
    base62::encode(v)
}

pub fn b62_to_u128(s: &str) -> Result<u128, String> {
    base62::decode(s).map_err(|e| format!("invalid base62: {e}"))
}

pub fn path_to_b62<T: PathElement>(path: &[T]) -> Result<String, String> {
    path_to_u128(path).map(u128_to_b62)
}

pub fn b62_to_path<T: PathElement>(s: &str) -> Result<Vec<T>, String> {
    b62_to_u128(s).and_then(u128_to_path)
}
