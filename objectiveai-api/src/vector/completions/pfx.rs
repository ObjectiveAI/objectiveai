//! Prefix tree for response key generation.
//!
//! Generates unique prefix keys (e.g., `` `A` ``, `` `B` ``) for labeling vector responses.
//! The LLM sees these keys and responds with its choice.
//!
//! The tree structure is designed around logprobs for probabilistic voting. Instead of
//! relying on the LLM's final sampled answer, we use logprobs to capture a probability
//! distribution over responses. The leaf width matches the number of logprobs the LLM
//! generates (e.g., 20 logprobs = 20 leaves per branch). For large response sets, nested
//! structures (`` `A` `` `` `A` ``, `` `A` `` `` `B` ``) allow capturing preferences across
//! more responses than a single logprobs batch allows.
//!
//! This enables probabilistic voting: LLMs are inherently probabilistic, and the sampler
//! makes the final discrete choice. By using logprobs, we bypass the sampler and capture
//! the model's full preference distribution.

use indexmap::IndexMap;
use rand::{Rng, seq::SliceRandom};
use std::sync::Arc;

/// Single-character prefix labels A-T.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pfx {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
}

impl Pfx {
    /// Converts this prefix to its character representation.
    pub fn to_char(&self) -> char {
        match self {
            Pfx::A => 'A',
            Pfx::B => 'B',
            Pfx::C => 'C',
            Pfx::D => 'D',
            Pfx::E => 'E',
            Pfx::F => 'F',
            Pfx::G => 'G',
            Pfx::H => 'H',
            Pfx::I => 'I',
            Pfx::J => 'J',
            Pfx::K => 'K',
            Pfx::L => 'L',
            Pfx::M => 'M',
            Pfx::N => 'N',
            Pfx::O => 'O',
            Pfx::P => 'P',
            Pfx::Q => 'Q',
            Pfx::R => 'R',
            Pfx::S => 'S',
            Pfx::T => 'T',
        }
    }

    /// Parses a character into a prefix, if valid.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'A' => Some(Pfx::A),
            'B' => Some(Pfx::B),
            'C' => Some(Pfx::C),
            'D' => Some(Pfx::D),
            'E' => Some(Pfx::E),
            'F' => Some(Pfx::F),
            'G' => Some(Pfx::G),
            'H' => Some(Pfx::H),
            'I' => Some(Pfx::I),
            'J' => Some(Pfx::J),
            'K' => Some(Pfx::K),
            'L' => Some(Pfx::L),
            'M' => Some(Pfx::M),
            'N' => Some(Pfx::N),
            'O' => Some(Pfx::O),
            'P' => Some(Pfx::P),
            'Q' => Some(Pfx::Q),
            'R' => Some(Pfx::R),
            'S' => Some(Pfx::S),
            'T' => Some(Pfx::T),
            _ => None,
        }
    }

    /// Returns all prefixes in randomized order.
    pub fn rng_vec(rng: &mut impl Rng) -> Vec<Self> {
        let mut vec = vec![
            Pfx::A,
            Pfx::B,
            Pfx::C,
            Pfx::D,
            Pfx::E,
            Pfx::F,
            Pfx::G,
            Pfx::H,
            Pfx::I,
            Pfx::J,
            Pfx::K,
            Pfx::L,
            Pfx::M,
            Pfx::N,
            Pfx::O,
            Pfx::P,
            Pfx::Q,
            Pfx::R,
            Pfx::S,
            Pfx::T,
        ];
        vec.shuffle(rng);
        vec
    }
}

impl std::fmt::Display for Pfx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// A tree structure for generating unique prefix keys.
///
/// The tree width is determined by the number of logprobs the LLM generates.
/// For flat structures (`` `A` ``, `` `B` ``), each leaf corresponds to one logprob slot.
/// For large response sets exceeding the logprobs limit, nested structures
/// (`` `A` `` `` `A` ``, `` `A` `` `` `B` ``) allow capturing preferences in stages.
#[derive(Debug, Clone)]
pub enum PfxTree {
    /// A branch containing child nodes.
    Branch(Arc<IndexMap<Pfx, PfxTree>>),
    /// A leaf containing the response index.
    Leaf(usize),
}

impl PfxTree {
    /// Creates a new prefix tree for the given number of responses.
    ///
    /// The `max_branch_len` should match the number of logprobs the LLM generates,
    /// ensuring each branch fits within one logprobs batch for probability capture.
    pub fn new(
        rng: &mut impl Rng,
        source_len: usize,
        max_branch_len: usize,
    ) -> Self {
        let mut source: Vec<usize> = (0..source_len).collect();
        source.shuffle(rng);
        Self::new_inner(rng, &source, max_branch_len, false)
    }

    /// Internal recursive constructor.
    pub fn new_inner(
        rng: &mut impl Rng,
        source: &[usize],
        max_branch_len: usize,
        force_sub_branch: bool,
    ) -> Self {
        let pfxs = Pfx::rng_vec(rng);
        if !force_sub_branch && source.len() <= max_branch_len {
            // return a single branch containing all leaves
            let mut branch = IndexMap::with_capacity(source.len());
            for (i, source_index) in source.iter().enumerate() {
                branch.insert(pfxs[i], PfxTree::Leaf(*source_index));
            }
            Self::Branch(Arc::new(branch))
        } else {
            // split into sub-branches
            let n = {
                let candidate =
                    (source.len() + max_branch_len - 1) / max_branch_len;
                if candidate <= max_branch_len {
                    candidate
                } else {
                    max_branch_len
                }
            };
            let base_per = source.len() / n;
            let extra = source.len() % n;
            let force_sub_branch =
                base_per + { if extra > 0 { 1 } else { 0 } } > max_branch_len;
            let mut branch = IndexMap::with_capacity(n);
            let mut i = 0;
            let mut count = 0;
            while i < n {
                let branch_len = base_per + if i < extra { 1 } else { 0 };
                branch.insert(
                    pfxs[i],
                    PfxTree::new_inner(
                        rng,
                        &source[count..count + branch_len],
                        max_branch_len,
                        force_sub_branch,
                    ),
                );
                count += branch_len;
                i += 1;
            }
            Self::Branch(Arc::new(branch))
        }
    }

    /// Generates prefix-to-index mappings in randomized order.
    ///
    /// Returns pairs of (prefix key, response index).
    pub fn pfx_indices(
        &self,
        rng: &mut impl Rng,
        source_len: usize,
    ) -> Vec<(String, usize)> {
        let mut indices = Vec::with_capacity(source_len);
        self.pfx_indices_inner(None, &mut indices);
        indices.shuffle(rng);
        indices
    }

    /// Internal recursive method for generating prefix indices.
    pub fn pfx_indices_inner(
        &self,
        parent_pfx: Option<String>,
        indices: &mut Vec<(String, usize)>,
    ) {
        match self {
            PfxTree::Branch(branch) => {
                for (pfx, child) in branch.as_ref() {
                    let parent_pfx = Some(match &parent_pfx {
                        Some(parent_pfx) => format!("{}`{}`", parent_pfx, pfx),
                        None => format!("`{}`", pfx),
                    });
                    child.pfx_indices_inner(parent_pfx, indices);
                }
            }
            PfxTree::Leaf(index) => {
                indices.push((parent_pfx.unwrap(), *index));
            }
        }
    }

    /// Gets a child node by prefix character.
    pub fn get(&self, pfx: Pfx) -> Option<PfxTree> {
        match self {
            PfxTree::Branch(branch) => branch.get(&pfx).cloned(),
            PfxTree::Leaf(_) => None,
        }
    }

    /// Returns the depth of the tree.
    pub fn depth(&self) -> usize {
        match self {
            PfxTree::Branch(branch) => {
                1 + branch
                    .values()
                    .next() // all sub-branches have the same depth
                    .map(|v| v.depth())
                    .unwrap_or(0)
            }
            PfxTree::Leaf(_) => 0,
        }
    }

    /// Unwraps a leaf node to get its response index.
    ///
    /// Panics if called on a branch node.
    pub fn unwrap_leaf(&self) -> usize {
        match self {
            PfxTree::Leaf(index) => *index,
            PfxTree::Branch(_) => {
                panic!("Called unwrap_leaf on a Branch")
            }
        }
    }

    /// Generates regex patterns for matching response keys.
    ///
    /// Returns (pattern with backticks, pattern without backticks).
    pub fn regex_patterns(&self, keys: &[(String, usize)]) -> (String, String) {
        let depth = self.depth();
        let mut with_ticks = String::with_capacity(
            (keys.len() - 1) // '|' characters
                + (keys.len() * depth * 3) // each key
                + keys.len() * 2, // parentheses
        );
        let mut without_ticks = String::with_capacity(
            (keys.len() - 1) // for '|' characters
                + keys.len() * (depth * 3 - 2) // each key stripped of ticks
                + keys.len() * 2, // parentheses
        );
        for (key, _) in keys {
            if with_ticks.len() > 0 {
                with_ticks.push('|');
                without_ticks.push('|');
            }
            with_ticks.push('(');
            without_ticks.push('(');
            with_ticks.push_str(key);
            without_ticks.push_str(&key[1..key.len() - 1]); // strip ticks
            with_ticks.push(')');
            without_ticks.push(')');
        }
        (with_ticks, without_ticks)
    }
}

/// Prefix data for a specific LLM, including tree and regex patterns.
#[derive(Debug, Clone)]
pub struct PfxData {
    /// The prefix tree for this LLM.
    pub pfx_tree: PfxTree,
    /// Regex pattern matching response keys with backticks.
    pub responses_key_pattern: String,
    /// Regex pattern matching response keys without backticks.
    pub responses_key_pattern_stripped: String,
}
