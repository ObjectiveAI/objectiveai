/// Optional structured payload for an abort. Mirrors `AbortReason` in
/// `abort.py:13-15` (`@dataclass(frozen=True)`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbortReason {
    pub message: String,
}
