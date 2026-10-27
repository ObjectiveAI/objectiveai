//! The one file out of a tar stream: what `podman cp` writes for a
//! single path.

/// The bytes of the first entry of `tar`, when it is a regular file;
/// `None` when it is anything else, or the stream ends first.
///
/// The rules, as a tar stream lays an entry out — a 512-byte header,
/// then the data padded to a multiple of 512:
///
/// 1. A header of all zero bytes ends the stream.
/// 2. The size is the octal number in bytes `124..136`, spaces and
///    NULs around it ignored. One that is not octal — the base-256 form
///    a size past eight gigabytes takes, among others — is no answer.
/// 3. The type is byte `156`. A PAX header (`x`, `g`) or a GNU long
///    name (`L`, `K`) describes the entry after it and is skipped, data
///    and padding together.
/// 4. A regular file (`0`, or NUL in the oldest form) is the answer:
///    its data, cut to the size. Any other type — a directory, a link
///    — is `None`.
/// 5. A header or data running past the stream's end is `None`.
pub(super) fn first_file(tar: &[u8]) -> Option<&[u8]> {
    const BLOCK: usize = 512;
    let mut at = 0;
    loop {
        let header = tar.get(at..at + BLOCK)?;
        if header.iter().all(|byte| *byte == 0) {
            return None;
        }
        let size = octal(header.get(124..136)?)?;
        let data = at + BLOCK;
        match header[156] {
            b'x' | b'g' | b'L' | b'K' => {
                at = data + size.div_ceil(BLOCK) * BLOCK;
            }
            b'0' | 0 => return tar.get(data..data + size),
            _ => return None,
        }
    }
}

/// An octal number written as ASCII digits, spaces and NULs around
/// it ignored; `None` for anything else.
fn octal(field: &[u8]) -> Option<usize> {
    let digits = field.iter().copied().filter(|byte| *byte != b' ' && *byte != 0);
    let mut value: usize = 0;
    let mut any = false;
    for digit in digits {
        if !(b'0'..=b'7').contains(&digit) {
            return None;
        }
        value = value.checked_mul(8)?.checked_add(usize::from(digit - b'0'))?;
        any = true;
    }
    any.then_some(value)
}
