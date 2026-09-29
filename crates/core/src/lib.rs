//! Deterministic Dating Copilot core (`dating_core`).
//!
//! This crate is the deterministic rule layer only. Concretely it provides:
//! - [`canonical`]: the `CanonicalMessage` schema and content-derived dedup.
//! - [`evidence`]: the evidence ledger, inferences and grounding checks.
//! - [`state`]: the relationship state machine and the adjudicator that rules
//!   on model-proposed transitions.
//! - [`nba`]: the deterministic next-best-action rule.
//!
//! Scope and boundaries (AGENTS.md rules 1, 2, 5, 7, 9, 10):
//! - No UI and no platform adapters live here. Adapters must not perform
//!   relationship inference; they only produce [`CanonicalMessage`] values.
//! - LLMs are allowed to *propose* transitions (see [`state::Proposal`]),
//!   but every decision is made by the deterministic functions in this crate.
//! - No persistence yet: everything operates on in-memory collections and is
//!   pure/deterministic (no I/O, no randomness, no wall-clock reads).
//! - Standard library only, zero external dependencies: the whole crate is
//!   the offline deterministic core.

pub mod canonical;
pub mod evidence;
pub mod nba;
pub mod state;

pub use canonical::{
    compute_dedupe_key, CanonicalError, CanonicalMessage, DedupeIndex, Direction, MessageType,
};
pub use evidence::{Evidence, FactType, Groundedness, Inference, Ledger, LedgerError};
pub use nba::{
    next_best_action, Advice, NbaError, NbaInput, NextBestAction, ReplyCandidate, ReplyStyle,
};
pub use state::{adjudicate, AdjudicationError, Applied, Proposal, Stage, Trend};
