use rand::Rng;
use rand::seq::SliceRandom;

use crate::agent::completions::message::{
    ImageUrl, ImageUrlDetail, RichContentPart,
};
use crate::functions::expression::{ImageInputSchema, InputValue};

pub const fn permutations(_schema: &ImageInputSchema) -> usize {
    4usize
}

pub fn generate<R: Rng>(
    _schema: &ImageInputSchema,
    mut rng: R,
) -> Generator<R> {
    let mut indices: Vec<usize> = (0..4).collect();
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
        let url = super::string::random_string(&mut self.rng);
        let detail = match index {
            0 => None,
            1 => Some(ImageUrlDetail::Auto),
            2 => Some(ImageUrlDetail::Low),
            _ => Some(ImageUrlDetail::High),
        };
        Some(InputValue::RichContentPart(RichContentPart::ImageUrl {
            image_url: ImageUrl { url, detail },
        }))
    }
}
