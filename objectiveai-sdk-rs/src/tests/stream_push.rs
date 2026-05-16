/// Defines a test that merges a vector of chunks by pushing them sequentially
/// into the first, then asserts the result equals the expected final chunk.
macro_rules! stream_push_test {
    ($name:ident, $chunks:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let mut chunks = $chunks;
            assert!(!chunks.is_empty(), "chunks must not be empty");
            let mut merged = chunks.remove(0);
            for chunk in &chunks {
                merged.push(chunk);
            }
            assert_eq!(merged, $expected);
        }
    };
}

pub(crate) use stream_push_test;
