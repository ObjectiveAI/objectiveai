use rand::Rng;
use rand::seq::SliceRandom;

use crate::agent::completions::message::{File, RichContentPart};
use crate::functions::expression::{FileInputSchema, InputValue};

pub const fn permutations(_schema: &FileInputSchema) -> usize {
    16usize
}

pub fn generate<R: Rng>(_schema: &FileInputSchema, mut rng: R) -> Generator<R> {
    let mut indices: Vec<usize> = (0..16).collect();
    indices.shuffle(&mut rng);
    Generator {
        indices,
        pos: 0,
        rng,
    }
}

pub struct Generator<R: Rng> {
    indices: Vec<usize>,
    pos: usize,
    rng: R,
}

impl<R: Rng> Iterator for Generator<R> {
    type Item = InputValue;
    fn next(&mut self) -> Option<InputValue> {
        if self.pos >= self.indices.len() {
            self.indices.shuffle(&mut self.rng);
            self.pos = 0;
        }
        let index = self.indices[self.pos];
        self.pos += 1;
        let mask = index as u32;
        let rng = &mut self.rng;
        let file = File {
            file_data: if mask & 1 != 0 {
                Some(super::string::random_string(rng))
            } else {
                None
            },
            file_id: if mask & 2 != 0 {
                Some(super::string::random_string(rng))
            } else {
                None
            },
            filename: if mask & 4 != 0 {
                Some(super::string::random_string(rng))
            } else {
                None
            },
            file_url: if mask & 8 != 0 {
                Some(super::string::random_string(rng))
            } else {
                None
            },
        };
        Some(InputValue::RichContentPart(RichContentPart::File { file }))
    }
}
