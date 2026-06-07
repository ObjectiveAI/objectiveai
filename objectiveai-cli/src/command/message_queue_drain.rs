//! Queue-drain join helper shared by `agents message` and
//! `agents spawn`. Both leaves drain the queue into a
//! `Vec<RichContent>` and then concatenate the drained items into
//! one combined `RichContent`, each item separated by a `"\n\n"`
//! text part.
//!
//! The implementation has two paths:
//!
//! * **All-text fast path** — when every queue item (and, for
//!   `agents message`, the user's own content) is plain text, we
//!   collapse to a single `RichContent::Text` with the separators
//!   inlined into the joined string. Cheap wire shape and matches
//!   what a human would expect when reading the resulting transcript.
//! * **Mixed-media path** — when any item carries image / audio /
//!   video / file parts, we build a `RichContent::Parts` vec with
//!   explicit `Text { text: "\n\n" }` separator parts inserted
//!   between items. We **don't** route through
//!   `RichContent::from(Vec<RichContentPart>)` here because its
//!   own all-text collapse joins consecutive text parts with
//!   `"\n\n"` — which would *double* our separators (turning
//!   `text1\n\ntext2` into `text1\n\n\n\ntext2`). The fast path
//!   above already handles the all-text case correctly, so
//!   bypassing the collapse for mixed inputs preserves intent.

use objectiveai_sdk::agent::completions::message::{RichContent, RichContentPart};

use crate::error::Error;

/// Combine the original spawn/message failure with the result of
/// `db::prompts::re_enqueue`. When re-enqueue succeeds, the original
/// error wins — the queue is whole; only delivery failed. When
/// re-enqueue *also* fails, both are surfaced via [`Error::DrainLost`]
/// so the caller sees what failed and that the queued content is lost.
///
/// `re_enqueue` returns `Result<(), crate::db::Error>`; this helper
/// handles the `From<crate::db::Error> for Error` conversion
/// internally so call sites can pass the raw `.await` result.
pub fn combine_drain_failure(
    original: Error,
    re_enqueue: Result<(), crate::db::Error>,
) -> Error {
    match re_enqueue {
        Ok(()) => original,
        Err(re_enqueue) => Error::DrainLost {
            original: Box::new(original),
            re_enqueue: Box::new(re_enqueue.into()),
        },
    }
}

/// `\n\n`-separated concatenation of `items` into one
/// [`RichContent`]. An empty input yields `RichContent::Text("")`;
/// a single-element input is returned unchanged (no separator
/// needed).
pub fn join_with_separator(items: Vec<RichContent>) -> RichContent {
    if items.is_empty() {
        return RichContent::Text(String::new());
    }
    // All-text fast path: collapse to a single Text with separators
    // inlined. Matches the human-readable transcript shape.
    if items.iter().all(|c| matches!(c, RichContent::Text(_))) {
        let joined = items
            .into_iter()
            .map(|c| match c {
                RichContent::Text(s) => s,
                RichContent::Parts(_) => unreachable!("all() guarded above"),
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        return RichContent::Text(joined);
    }
    // Mixed-media path: build Parts with explicit separator parts
    // between items. Do not route through `RichContent::from` — its
    // all-text collapse would double the separators.
    let mut parts: Vec<RichContentPart> = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            parts.push(RichContentPart::Text { text: "\n\n".to_string() });
        }
        match item {
            RichContent::Text(s) => parts.push(RichContentPart::Text { text: s }),
            RichContent::Parts(p) => parts.extend(p),
        }
    }
    RichContent::Parts(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use objectiveai_sdk::agent::completions::message::{ImageUrl, RichContentPart};

    #[test]
    fn empty_input_is_empty_text() {
        assert_eq!(join_with_separator(vec![]), RichContent::Text(String::new()));
    }

    #[test]
    fn single_text_passes_through() {
        let got = join_with_separator(vec![RichContent::Text("hi".into())]);
        assert_eq!(got, RichContent::Text("hi".into()));
    }

    #[test]
    fn all_text_inlines_separators() {
        let got = join_with_separator(vec![
            RichContent::Text("a".into()),
            RichContent::Text("b".into()),
            RichContent::Text("c".into()),
        ]);
        assert_eq!(got, RichContent::Text("a\n\nb\n\nc".into()));
    }

    #[test]
    fn mixed_media_keeps_separator_parts() {
        let image = RichContentPart::ImageUrl {
            image_url: ImageUrl { url: "data:image/png;base64,Z".into(), detail: None },
        };
        let got = join_with_separator(vec![
            RichContent::Parts(vec![image.clone()]),
            RichContent::Text("describe".into()),
        ]);
        let RichContent::Parts(parts) = got else {
            panic!("expected Parts for mixed-media input");
        };
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], image);
        assert_eq!(parts[1], RichContentPart::Text { text: "\n\n".into() });
        assert_eq!(parts[2], RichContentPart::Text { text: "describe".into() });
    }
}
