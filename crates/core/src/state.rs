//! Relationship state machine and the deterministic adjudicator.
//!
//! Stage and Trend are separate axes (ARCHITECTURE.md "Relationship"). An LLM
//! may only emit a [`Proposal`]; [`adjudicate`] is the sole authority that
//! decides what is applied (AGENTS.md rule 5). It is a pure function of its
//! inputs: no clock, no randomness, no persistence.

use std::fmt;

/// Relationship stage, declared in ascending order of closeness.
///
/// The discriminants double as the rank ordering used for forward/backward
/// move checks; keep the variant order in sync with ARCHITECTURE.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    Unknown,
    Acquaintance,
    ActiveChat,
    Familiar,
    Warming,
    Flirting,
    InvitationReady,
    DatePlanned,
    Dated,
    PostDate,
}

impl Stage {
    /// Position in the ladder (0-based, in declaration order).
    pub fn rank(self) -> usize {
        self as usize
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Unknown => "UNKNOWN",
            Stage::Acquaintance => "ACQUAINTANCE",
            Stage::ActiveChat => "ACTIVE_CHAT",
            Stage::Familiar => "FAMILIAR",
            Stage::Warming => "WARMING",
            Stage::Flirting => "FLIRTING",
            Stage::InvitationReady => "INVITATION_READY",
            Stage::DatePlanned => "DATE_PLANNED",
            Stage::Dated => "DATED",
            Stage::PostDate => "POST_DATE",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Direction of the relationship's recent momentum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trend {
    Warming,
    Stable,
    Cooling,
    Boundary,
}

impl Trend {
    pub fn as_str(self) -> &'static str {
        match self {
            Trend::Warming => "WARMING",
            Trend::Stable => "STABLE",
            Trend::Cooling => "COOLING",
            Trend::Boundary => "BOUNDARY",
        }
    }
}

impl fmt::Display for Trend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A model-proposed transition. Never applied directly — see [`adjudicate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub from: Stage,
    pub to: Stage,
    pub trend: Trend,
    /// Human-readable rationale from the model. Stored for audit only; the
    /// adjudicator never parses it.
    pub reason: String,
    pub evidence_ids: Vec<String>,
}

/// The deterministic result of an accepted proposal: the applied stage and
/// trend. Deliberately *not* a "completion" or success verdict — the state
/// simply moved to `stage` with `trend`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    pub stage: Stage,
    pub trend: Trend,
}

/// Deterministic reasons a proposal is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdjudicationError {
    /// No evidence cited — violates traceability (AGENTS.md rule 7).
    NoEvidenceIds,
    /// A cited evidence id is not known to the ledger; the id is reported.
    UnknownEvidence(String),
    /// A forward move of more than one rank (no skip-ahead).
    SkipForward {
        from: Stage,
        to: Stage,
        distance: usize,
    },
    /// A boundary signal is present, so any forward move is forbidden
    /// (AGENTS.md rule 9).
    BoundaryBlocksForward { to: Stage },
}

impl fmt::Display for AdjudicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdjudicationError::NoEvidenceIds => {
                write!(f, "proposal cites no evidence ids (traceability required)")
            }
            AdjudicationError::UnknownEvidence(id) => {
                write!(f, "proposal references unknown evidence `{id}`")
            }
            AdjudicationError::SkipForward { from, to, distance } => {
                write!(
                    f,
                    "forward move {from} -> {to} skips {distance} ranks (> 1)"
                )
            }
            AdjudicationError::BoundaryBlocksForward { to } => {
                write!(f, "boundary signal blocks forward move to {to}")
            }
        }
    }
}

/// Adjudicate one proposed transition with fixed, deterministic rule order:
/// 1. empty `evidence_ids` ⇒ [`AdjudicationError::NoEvidenceIds`];
/// 2. any cited id the ledger does not know ⇒
///    [`AdjudicationError::UnknownEvidence`] (names the first bad id);
/// 3. `boundary_present` plus any forward move ⇒
///    [`AdjudicationError::BoundaryBlocksForward`] (AGENTS.md rule 9);
/// 4. forward move of more than one rank ⇒ [`AdjudicationError::SkipForward`];
/// 5. otherwise the move is applied: forward one rank, lateral, or any
///    backward move.
///
/// The applied trend always carries the proposal's trend, so a `Boundary`
/// trend survives regardless of which stage move (if any) was allowed.
pub fn adjudicate(
    proposal: &Proposal,
    boundary_present: bool,
    ledger_has_evidence: impl Fn(&str) -> bool,
) -> Result<Applied, AdjudicationError> {
    if proposal.evidence_ids.is_empty() {
        return Err(AdjudicationError::NoEvidenceIds);
    }
    for id in &proposal.evidence_ids {
        if !ledger_has_evidence(id) {
            return Err(AdjudicationError::UnknownEvidence(id.clone()));
        }
    }

    let is_forward = proposal.to.rank() > proposal.from.rank();
    if is_forward {
        if boundary_present {
            return Err(AdjudicationError::BoundaryBlocksForward { to: proposal.to });
        }
        let distance = proposal.to.rank() - proposal.from.rank();
        if distance > 1 {
            return Err(AdjudicationError::SkipForward {
                from: proposal.from,
                to: proposal.to,
                distance,
            });
        }
    }

    Ok(Applied {
        stage: proposal.to,
        trend: proposal.trend,
    })
}
