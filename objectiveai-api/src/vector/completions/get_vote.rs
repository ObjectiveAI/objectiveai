//! Vote extraction from LLM responses.
//!
//! Extracts votes from LLM chat completion responses by parsing response keys
//! and computing probability distributions from logprobs when available.

use regex::Regex;
use rust_decimal::MathematicalOps;

/// Extracts a vote from an LLM choice.
///
/// Parses the response to find selected response keys and converts them into a
/// probability distribution. When logprobs are available, uses them to capture
/// the model's preference distribution (probabilistic voting). Otherwise, falls
/// back to discrete voting based on the final sampled token.
///
/// Returns None if no response key is found in the content.
pub fn get_vote(
    mut pfx_tree: super::PfxTree,
    with_ticks_pattern: &str,
    without_ticks_pattern: &str,
    responses_len: usize,
    choice: &objectiveai::chat::completions::response::streaming::Choice,
) -> Option<Vec<rust_decimal::Decimal>> {
    // extract content, return None if empty
    let content_owned = match Content::from_choice(choice) {
        Some(c) => c,
        None => {
            return None;
        }
    };
    let content = content_owned.as_str();

    // extract response keys, return if not found
    let with_ticks_re = Regex::new(with_ticks_pattern).unwrap();
    let mut key_matches = with_ticks_re.find_iter(content).collect::<Vec<_>>();
    let without_ticks_re = match key_matches.len() {
        0 => Some(Regex::new(without_ticks_pattern).unwrap()),
        _ => None,
    };
    if let Some(without_ticks_re) = without_ticks_re.as_ref() {
        key_matches = without_ticks_re.find_iter(content).collect::<Vec<_>>();
    }

    // return None if no keys found
    if key_matches.is_empty() {
        return None;
    }

    // each match has an equal vote weight
    let key_matches_len_decimal =
        rust_decimal::Decimal::from(key_matches.len());

    // reverse matches for processing
    let keys_rev = key_matches
        .into_iter()
        .rev()
        .map(|cap| cap.as_str())
        .collect::<Vec<_>>();

    // prepare vote
    let mut vote = vec![rust_decimal::Decimal::ZERO; responses_len];

    // track logprob index
    let mut logprob_i = 0;

    for key in keys_rev {
        // get the final prefix
        let (final_pfx_char, final_pfx) = key
            .chars()
            .rev()
            .map(|c| (c, super::Pfx::from_char(c)))
            .filter(|(_, pfx)| pfx.is_some())
            .next()
            .unwrap();
        let final_pfx = final_pfx.unwrap();

        // get to the lowest pfx tree branch
        let mut i = pfx_tree.depth() - 1;
        if i > 0 {
            for c in key.chars() {
                if let Some(pfx) = super::Pfx::from_char(c) {
                    pfx_tree = pfx_tree.get(pfx).unwrap();
                    i -= 1;
                    if i == 0 {
                        break;
                    }
                }
            }
        }
        let pfx_tree = match pfx_tree.clone() {
            super::PfxTree::Branch(branch) => branch,
            super::PfxTree::Leaf(_) => unreachable!(),
        };

        // try to get probabilities from logprobs
        let mut from_logprobs = false;
        if let Some(objectiveai::chat::completions::response::Logprobs {
            content: Some(logprobs),
            ..
        }) = choice.logprobs.as_ref()
        {
            // reverse key to check against
            let key_rev = key.chars().rev().collect::<String>();

            // slice as we go
            let mut key_rev_slice = key_rev.as_str();

            // keep the relevant logprob
            let mut key_logprob = None;
            let mut key_logprob_index = 0;

            // find the logprob segment that matches the key
            'outer: for logprob in logprobs.iter().rev().skip(logprob_i) {
                logprob_i += 1;
                let mut i = logprob.token.len();
                for c in logprob.token.chars().rev() {
                    i -= c.len_utf8();
                    if key_rev_slice.starts_with(c) {
                        // match
                        // remove the matched char from the slice
                        key_rev_slice = &key_rev_slice[c.len_utf8()..];
                        // keep the logprob that contains the final pfx
                        if key_logprob.is_none() && c == final_pfx_char {
                            key_logprob = Some(logprob);
                            key_logprob_index = i;
                        }
                        // stop when the full match is found
                        if key_rev_slice.is_empty() {
                            break 'outer;
                        }
                    } else if key_rev_slice.len() != key_rev.len() {
                        // not match
                        // reset
                        key_rev_slice = key_rev.as_str();
                        key_logprob = None;
                        key_logprob_index = 0;
                    } else {
                        // unknown
                    }
                }
            }

            // matching logprob segment found
            if key_rev_slice.is_empty() {
                // collect probabilities
                let mut probabilities =
                    vec![rust_decimal::Decimal::ZERO; responses_len];
                let mut probabilities_sum = rust_decimal::Decimal::ZERO;
                for objectiveai::chat::completions::response::TopLogprob {
                    token,
                    logprob,
                    ..
                } in &key_logprob.as_ref().unwrap().top_logprobs
                {
                    if key_logprob_index < token.len()
                        && let Some(logprob) = logprob
                        && let Some((_, c)) = token
                            .char_indices()
                            .find(|(i, _)| *i == key_logprob_index)
                        && let Some(pfx) = super::Pfx::from_char(c)
                        && let Some(leaf) = pfx_tree.get(&pfx)
                    {
                        // logprobs sourced vote successful
                        from_logprobs = true;

                        // add to probabilities
                        let probability = logprob.exp();
                        probabilities[leaf.unwrap_leaf()] += probability;
                        probabilities_sum += probability;
                    }
                }

                // normalize and add to vote
                if probabilities_sum > rust_decimal::Decimal::ZERO {
                    let mut vote_i = 0;
                    while vote_i < vote.len() {
                        vote[vote_i] += (probabilities[vote_i]
                            / probabilities_sum)
                            / key_matches_len_decimal;
                        vote_i += 1;
                    }
                }
            }
        }

        // fallback, set vote indexed to selected response to 1.0
        if !from_logprobs {
            vote[pfx_tree.get(&final_pfx).unwrap().unwrap_leaf()] =
                rust_decimal::Decimal::ONE / key_matches_len_decimal;
        }
    }

    // return vote
    Some(vote)
}

/// Helper for extracting content from choices without unnecessary allocation.
enum Content<'s> {
    /// Borrowed content from choice.delta.content.
    Ref(&'s str),
    /// Owned content when combining tool call arguments with content.
    Owned(String),
}

impl<'s> Content<'s> {
    /// Returns the content as a string slice.
    fn as_str(&self) -> &str {
        match self {
            Content::Ref(s) => s,
            Content::Owned(s) => s.as_str(),
        }
    }

    /// Extracts content from a choice, combining tool call arguments if present.
    fn from_choice(
        choice: &'s objectiveai::chat::completions::response::streaming::Choice,
    ) -> Option<Self> {
        match choice.delta.tool_calls.as_ref() {
            Some(tool_calls) => {
                let mut len = 0;
                for tool_call in tool_calls {
                    if let Some(
                    objectiveai::chat::completions::response::streaming::ToolCallFunction {
                        arguments: Some(arguments),
                        ..
                    },
                ) = tool_call.function.as_ref()
                {
                    len += arguments.len();
                }
                }
                if let Some(content) = choice.delta.content.as_ref() {
                    len += content.len();
                }
                if len == 0 {
                    return None;
                }
                let mut owned = String::with_capacity(len);
                for tool_call in tool_calls {
                    if let Some(
                    objectiveai::chat::completions::response::streaming::ToolCallFunction {
                        arguments: Some(arguments),
                        ..
                    },
                ) = tool_call.function.as_ref()
                {
                    owned.push_str(arguments);
                }
                }
                if let Some(content) = choice.delta.content.as_ref() {
                    owned.push_str(content);
                }
                Some(Content::Owned(owned))
            }
            None => {
                if let Some(content) = choice.delta.content.as_ref()
                    && content.len() > 0
                {
                    Some(Content::Ref(content.as_str()))
                } else {
                    None
                }
            }
        }
    }
}
