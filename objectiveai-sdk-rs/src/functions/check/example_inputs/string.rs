use rand::Rng;
use rand::seq::SliceRandom;

use crate::functions::expression::{InputValue, StringInputSchema};

pub fn permutations(schema: &StringInputSchema) -> usize {
    if let Some(ref e) = schema.r#enum {
        e.len()
    } else {
        2
    }
}

pub fn generate<R: Rng>(
    schema: &StringInputSchema,
    mut rng: R,
) -> Generator<R> {
    let count = permutations(schema);
    let mut indices: Vec<usize> = (0..count).collect();
    indices.shuffle(&mut rng);
    let variants = if let Some(ref e) = schema.r#enum {
        Some(e.clone())
    } else {
        None
    };
    Generator {
        variants,
        indices,
        pos: 0,
        rng,
    }
}

pub struct Generator<R: Rng> {
    variants: Option<Vec<String>>,
    indices: Vec<usize>,
    pos: usize,
    rng: R,
}

impl<R: Rng> Iterator for Generator<R> {
    type Item = InputValue;
    fn next(&mut self) -> Option<InputValue> {
        if self.indices.is_empty() {
            return None;
        }
        if self.pos >= self.indices.len() {
            self.indices.shuffle(&mut self.rng);
            self.pos = 0;
        }
        let index = self.indices[self.pos];
        self.pos += 1;
        Some(if let Some(ref e) = self.variants {
            InputValue::String(e[index].clone())
        } else if index == 0 {
            InputValue::String(String::new())
        } else {
            InputValue::String(random_string(&mut self.rng))
        })
    }
}

pub fn random_string(rng: &mut impl Rng) -> String {
    let len = rng.random_range(1..=32);
    (0..len)
        .map(|_| rng.random_range(b'a'..=b'z') as char)
        .collect()
}
