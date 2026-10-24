//! The user code a person types.

use rand::Rng as _;

/// The characters a user code is drawn from: consonants alone, as RFC
/// 8628 §6.1 suggests, so no code spells a word and none is mistaken
/// for a digit.
const ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";

/// The characters in a code.
const LENGTH: usize = 8;

/// A new user code: eight characters of [`ALPHABET`], about 35 bits.
pub fn draw() -> String {
    let mut rng = rand::rng();
    (0..LENGTH).map(|_| char::from(ALPHABET[rng.random_range(0..ALPHABET.len())])).collect()
}

/// The code `typed` is, as a person typed it: letters in either case,
/// with any spaces and hyphens between them. `None` when what is left
/// is not eight characters of the alphabet.
pub fn read(typed: &str) -> Option<String> {
    let code: String = typed.chars().filter(|c| !matches!(c, ' ' | '-')).map(|c| c.to_ascii_uppercase()).collect();
    (code.len() == LENGTH && code.bytes().all(|byte| ALPHABET.contains(&byte))).then_some(code)
}

/// The code as a person is shown it: two halves joined by a hyphen.
pub fn show(code: &str) -> String {
    let (first, second) = code.split_at(code.len() / 2);
    format!("{first}-{second}")
}
