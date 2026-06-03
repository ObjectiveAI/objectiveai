use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

use crate::functions::expression::{InputSchema, InputValue};

pub fn permutations(schema: &InputSchema) -> usize {
    inner_permutations(schema) * 2
}

pub fn inner_permutations(schema: &InputSchema) -> usize {
    match schema {
        InputSchema::Boolean(s) => super::boolean::permutations(s),
        InputSchema::String(s) => super::string::permutations(s),
        InputSchema::Integer(s) => super::integer::permutations(s),
        InputSchema::Number(s) => super::number::permutations(s),
        InputSchema::Image(s) => super::image::permutations(s),
        InputSchema::Audio(s) => super::audio::permutations(s),
        InputSchema::Video(s) => super::video::permutations(s),
        InputSchema::File(s) => super::file::permutations(s),
        InputSchema::Object(s) => super::object::permutations(s),
        InputSchema::Array(s) => super::array::permutations(s),
        InputSchema::AnyOf(s) => super::any_of::permutations(s),
    }
}

pub fn generate(schema: &InputSchema, mut rng: StdRng) -> Generator {
    let inner_count = inner_permutations(schema);
    // 0..inner_count = present, inner_count..inner_count*2 = absent
    let total = inner_count * 2;
    let mut indices: Vec<usize> = (0..total).collect();
    indices.shuffle(&mut rng);

    let inner = super::multi::generate(
        schema,
        StdRng::seed_from_u64(rng.random::<u64>()),
    );

    Generator {
        inner,
        inner_count,
        indices,
        pos: 0,
        rng,
    }
}

pub struct Generator {
    inner: super::multi::Generator,
    inner_count: usize,
    indices: Vec<usize>,
    pos: usize,
    rng: StdRng,
}

impl Iterator for Generator {
    type Item = Option<InputValue>;
    fn next(&mut self) -> Option<Option<InputValue>> {
        if self.indices.is_empty() {
            return Some(None);
        }
        if self.pos >= self.indices.len() {
            self.indices.shuffle(&mut self.rng);
            self.pos = 0;
        }
        let index = self.indices[self.pos];
        self.pos += 1;
        if index < self.inner_count {
            Some(self.inner.next())
        } else {
            Some(None)
        }
    }
}
