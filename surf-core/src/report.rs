//! The `surf check --format json` contract (§5). This is the seam every optional layer
//! (reviewer plugin, etc.) attaches to; the deterministic core never depends on it.
//!
//! The verdict is carried by `old_hash` vs `new_hash`. `old_code` and `magnitude` are
//! best-effort enrichment recovered from the previous source via git — advisory only,
//! `None` when unavailable, and never part of the pass/fail decision.

use crate::hash::Magnitude;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceKind {
    /// Stored hash exists and no longer matches the current span.
    Changed,
    /// The claim has never been verified — no stored hash to compare against.
    Unverified,
    /// The anchor no longer resolves to exactly one symbol (run `surf lint`).
    Unresolvable,
}

#[derive(Debug, Clone, Serialize)]
pub struct Divergence {
    pub hub: String,
    pub claim: String,
    pub at: String,
    pub kind: DivergenceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_code: Option<String>,
    pub prose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<Magnitude>,
}
