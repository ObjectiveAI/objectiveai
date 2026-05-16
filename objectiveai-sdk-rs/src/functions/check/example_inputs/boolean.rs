use rand::Rng;
use rand::seq::SliceRandom;

use crate::functions::expression::{BooleanInputSchema, InputValue};

pub const fn permutations(_schema: &BooleanInputSchema) -> usize {
    2usize
}

pub fn generate<R: Rng>(
    _schema: &BooleanInputSchema,
    mut rng: R,
) -> Generator<R> {
    let mut indices: Vec<usize> = (0..2).collect();
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
        Some(InputValue::Boolean(index == 0))
    }
}
