pub mod any_of;
pub mod array;
pub mod audio;
pub mod boolean;
pub mod file;
pub mod image;
pub mod integer;
pub mod multi;
pub mod number;
pub mod object;
pub mod optional;
pub mod string;
pub mod video;

#[cfg(test)]
mod array_tests;
#[cfg(test)]
mod tests;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::functions::expression::{InputValue, InputSchema};

pub fn permutations(schema: &InputSchema) -> usize {
    optional::inner_permutations(schema)
}

pub fn generate(schema: &InputSchema) -> Generator {
    generate_seeded(schema, StdRng::from_os_rng())
}

pub fn generate_seeded(schema: &InputSchema, rng: StdRng) -> Generator {
    let perms = permutations(schema);
    Generator {
        inner: multi::generate(schema, rng),
        remaining: perms,
    }
}

pub struct Generator {
    inner: multi::Generator,
    remaining: usize,
}

impl Iterator for Generator {
    type Item = InputValue;
    fn next(&mut self) -> Option<InputValue> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.inner.next()
    }
}
