//! Stream-consumption helpers for snapshot tests. Moved verbatim from
//! the old `objectiveai-api/src/stream_harness.rs`; visibility flipped
//! from `pub(crate)` to `pub` since each integration test binary
//! includes this via `mod common;`.
//!
//! The 2-behind buffer pattern lets tests distinguish the last /
//! second-to-last / other chunks without collecting the entire stream
//! into a `Vec`.

use futures::StreamExt;

/// Consume a stream of chunks using a 2-behind buffer.
pub async fn consume_stream<C, S>(
    stream: S,
    mut push: impl FnMut(&mut C, &C),
    mut on_chunk: impl FnMut(usize, &C),
    mut on_second_to_last: impl FnMut(usize, &C),
    mut on_last: impl FnMut(usize, &C),
) -> C
where
    C: Clone,
    S: futures::Stream<Item = C> + Unpin,
{
    let mut stream = stream;

    let mut agg: Option<C> = None;
    let mut buf: (Option<(usize, C)>, Option<(usize, C)>) = (None, None);
    let mut idx: usize = 0;

    while let Some(chunk) = stream.next().await {
        match &mut agg {
            Some(a) => push(a, &chunk),
            None => agg = Some(chunk.clone()),
        }

        if let (Some((pp_idx, pp)), _) = &buf {
            on_chunk(*pp_idx, pp);
        }
        buf = (buf.1.take(), Some((idx, chunk)));
        idx += 1;
    }

    match buf {
        (Some((pp_idx, pp)), Some((p_idx, p))) => {
            on_second_to_last(pp_idx, &pp);
            on_last(p_idx, &p);
        }
        (None, Some((p_idx, p))) => {
            on_last(p_idx, &p);
        }
        _ => panic!("stream must produce at least one chunk"),
    }

    agg.expect("stream must produce at least one chunk")
}

/// Like [`consume_stream`], but for streams whose items are not
/// directly the chunk type (e.g. `StreamItem<STATE>` wraps chunks
/// alongside state items). `extract` filters non-chunk items.
pub async fn consume_stream_items<C, I, S>(
    stream: S,
    mut extract: impl FnMut(I) -> Option<C>,
    mut push: impl FnMut(&mut C, &C),
    mut on_chunk: impl FnMut(usize, &C),
    mut on_second_to_last: impl FnMut(usize, &C),
    mut on_last: impl FnMut(usize, &C),
) -> C
where
    C: Clone,
    S: futures::Stream<Item = I> + Unpin,
{
    let mut stream = stream;

    let mut agg: Option<C> = None;
    let mut buf: (Option<(usize, C)>, Option<(usize, C)>) = (None, None);
    let mut idx: usize = 0;
    let mut saw_non_chunk = false;

    while let Some(item) = stream.next().await {
        match extract(item) {
            Some(chunk) => {
                match &mut agg {
                    Some(a) => push(a, &chunk),
                    None => agg = Some(chunk.clone()),
                }

                if let (Some((pp_idx, pp)), _) = &buf {
                    on_chunk(*pp_idx, pp);
                }
                buf = (buf.1.take(), Some((idx, chunk)));
                idx += 1;
            }
            None => {
                saw_non_chunk = true;
            }
        }
    }

    assert!(saw_non_chunk, "stream must contain at least one non-chunk item (e.g. State)");

    match buf {
        (Some((pp_idx, pp)), Some((p_idx, p))) => {
            on_second_to_last(pp_idx, &pp);
            on_last(p_idx, &p);
        }
        (None, Some((p_idx, p))) => {
            on_last(p_idx, &p);
        }
        _ => panic!("stream must produce at least one chunk"),
    }

    agg.expect("stream must produce at least one chunk")
}

/// Like [`consume_stream`], but with an accumulator built across all
/// chunks and passed to `on_last` for richer assertion messages.
pub async fn consume_stream_acc<C, S, A>(
    stream: S,
    mut push: impl FnMut(&mut C, &C),
    mut accumulate: impl FnMut(&C, &mut A),
    mut on_chunk: impl FnMut(usize, &C),
    mut on_second_to_last: impl FnMut(usize, &C),
    mut on_last: impl FnMut(usize, &C, &A),
    mut acc: A,
) -> C
where
    C: Clone,
    S: futures::Stream<Item = C> + Unpin,
{
    let mut stream = stream;

    let mut agg: Option<C> = None;
    let mut buf: (Option<(usize, C)>, Option<(usize, C)>) = (None, None);
    let mut idx: usize = 0;

    while let Some(chunk) = stream.next().await {
        match &mut agg {
            Some(a) => push(a, &chunk),
            None => agg = Some(chunk.clone()),
        }

        accumulate(&chunk, &mut acc);

        if let (Some((pp_idx, pp)), _) = &buf {
            on_chunk(*pp_idx, pp);
        }
        buf = (buf.1.take(), Some((idx, chunk)));
        idx += 1;
    }

    match buf {
        (Some((pp_idx, pp)), Some((p_idx, p))) => {
            on_second_to_last(pp_idx, &pp);
            on_last(p_idx, &p, &acc);
        }
        (None, Some((p_idx, p))) => {
            on_last(p_idx, &p, &acc);
        }
        _ => panic!("stream must produce at least one chunk"),
    }

    agg.expect("stream must produce at least one chunk")
}

/// Shared snapshot assertion. When `env_var` is set to `"1"`, writes
/// `json` to `path` (update mode); otherwise asserts `json == expected`.
pub fn assert_snapshot(json: &str, path: &str, expected: &str, env_var: &str) {
    if std::env::var(env_var).as_deref() == Ok("1") {
        std::fs::write(path, json).unwrap();
        eprintln!("Updated snapshot: {path}");
        let written = std::fs::read_to_string(path).unwrap();
        assert_eq!(json, written.trim_end());
    } else {
        assert_eq!(json, expected.trim_end());
    }
}
