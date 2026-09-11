//! Tool call ids: random, and not hex.

use uuid::Uuid;

/// The digits, in the order that sorts as ASCII does: numerals,
/// lower case, upper case.
const ALPHABET: &[u8; 62] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// How many base-62 digits 128 bits need: 62^22 exceeds 2^128, and
/// 62^21 does not.
const DIGITS: usize = 22;

/// A fresh tool call id: 128 random bits in base 62, 22 characters
/// — a third shorter than the hex a uuid prints as, for the same
/// entropy. The gateway's stream carries no ids of its own, so the
/// container mints them; nothing but this run ever has to read one.
pub fn new() -> String {
    let mut bits = Uuid::new_v4().as_u128();
    let mut digits = [ALPHABET[0]; DIGITS];
    for digit in digits.iter_mut().rev() {
        *digit = ALPHABET[(bits % 62) as usize];
        bits /= 62;
    }
    String::from_utf8(digits.to_vec()).expect("the alphabet is ASCII")
}
