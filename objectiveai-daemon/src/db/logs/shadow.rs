//! Shadow map for streaming-content rows.
//!
//! Keyed by [`OwnedRowKey`] (one variant per streaming-content
//! table), valued by [`RowBody`] (the last-written body). On every
//! incoming [`RowValue`]:
//!
//! 1. The borrowed [`RowKey`] is hashed and the shadow probed via
//!    hashbrown's `raw_entry_mut` — no allocation in the Skip case.
//! 2. If a row is found, [`RowValue::body_eq`] field-compares the
//!    new body against the stored one. Fast bail on length / first-
//!    byte mismatch, no fingerprint hash needed.
//! 3. The verdict is returned as [`WriteOp::Insert`] / `Update` /
//!    `Skip`. The writer dispatches the matching flat SQL — no
//!    `ON CONFLICT` clauses, no upsert ambiguity.
//!
//! Allocations are confined to the Insert + Update paths: a single
//! `to_owned_key()` (response_id String clone) on Insert, and a
//! `to_body()` on either Insert or Update. The Skip path
//! (overwhelmingly common in steady-state streaming) touches zero
//! heap.

use std::hash::{BuildHasher, Hash, Hasher};

use hashbrown::HashMap;
use hashbrown::hash_map::RawEntryMut;

use super::row::{OwnedRowKey, RowBody, RowValue};

/// What the writer should do with a particular row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOp {
    Insert,
    Update,
    Skip,
}

#[derive(Default)]
pub struct Shadow {
    rows: HashMap<OwnedRowKey, RowBody>,
}

impl Shadow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Probe the shadow for `value`'s row and decide whether to write.
    ///
    /// Updates the shadow's stored body on Insert / Update. Returns
    /// the verdict for the writer to dispatch.
    pub fn record(&mut self, value: &RowValue<'_>) -> WriteOp {
        let key_ref = value.key();

        // Compute the hash from the borrowed key. Identical to the
        // hash an owned key with the same fields would produce, since
        // `&str` and `String` share the same Hash impl.
        let hash = {
            let mut state = self.rows.hasher().build_hasher();
            key_ref.hash(&mut state);
            state.finish()
        };

        let entry = self
            .rows
            .raw_entry_mut()
            .from_hash(hash, |owned| key_ref.matches_owned(owned));

        match entry {
            RawEntryMut::Occupied(mut o) => {
                if value.body_eq(o.get()) {
                    WriteOp::Skip
                } else {
                    *o.get_mut() = value.to_body();
                    WriteOp::Update
                }
            }
            RawEntryMut::Vacant(v) => {
                // We already computed `hash` via the table's hasher
                // above; `insert_hashed_nocheck` stores the entry at
                // that hash and lets the table rehash with its own
                // BuildHasher on resize.
                v.insert_hashed_nocheck(hash, key_ref.to_owned_key(), value.to_body());
                WriteOp::Insert
            }
        }
    }
}
