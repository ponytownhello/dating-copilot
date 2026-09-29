//! Next-best-action deterministic rule.
//!
//! [`NextBestAction`] is the recommended move (ARCHITECTURE.md "Next Best
//! Action"). This module is pure: given an [`NbaInput`] it returns an
//! [`Advice`]. No model call, no randomness, no I/O, and no numeric
//! "likability percentage" anywhere — the docs forbid false precision, so
//! every reason cites evidence ids instead.
//!
//! AGENTS.md rule 8: nothing here sends a message. [`Advice`] only carries
//! copyable reply candidates; dispatch is always the user's decision.
//! AGENTS.md rule 9 + PROMPT_DESIGN.md "Reply": an explicit rejection yields
//! [`NextBestAction::Stop`] and any boundary signal yields
//! [`NextBestAction::CoolDown`], both dominating every stage-driven
//! suggestion; after either outcome no `GentleProgress`/invite-flavoured
//! copy may be generated.

use crate::evidence::{FactType, Ledger};
use crate::state::Stage;
use std::fmt;

/// The recommended next move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextBestAction {
    Wait,
    Continue,
    Change,
    Flirt,
    Invite,
    Confirm,
    FollowUp,
    CoolDown,
    Stop,
}

impl NextBestAction {
    pub fn as_str(self) -> &'static str {
        match self {
            NextBestAction::Wait => "WAIT",
            NextBestAction::Continue => "CONTINUE",
            NextBestAction::Change => "CHANGE",
            NextBestAction::Flirt => "FLIRT",
            NextBestAction::Invite => "INVITE",
            NextBestAction::Confirm => "CONFIRM",
            NextBestAction::FollowUp => "FOLLOW_UP",
            NextBestAction::CoolDown => "COOL_DOWN",
            NextBestAction::Stop => "STOP",
        }
    }
}

impl fmt::Display for NextBestAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Reply tone buckets (PROMPT_DESIGN.md "Reply": natural / humorous /
/// gentle_progress). The core does not infer text safety from the tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyStyle {
    Natural,
    Humorous,
    GentleProgress,
}

/// One copyable reply candidate produced by the prompt layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyCandidate {
    pub style: ReplyStyle,
    pub text: String,
}

/// The full advice payload produced by [`next_best_action`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advice {
    pub action: NextBestAction,
    pub reason: String,
    pub evidence_ids: Vec<String>,
    pub avoid: Vec<String>,
    pub reply_candidates: Vec<ReplyCandidate>,
}

/// The NBA cannot return a recommendation unless it can bind that
/// recommendation to evidence that still exists in the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NbaError {
    NoEvidenceIds,
    UnknownEvidence(String),
}

impl fmt::Display for NbaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NbaError::NoEvidenceIds => {
                f.write_str("next-best-action requires at least one evidence id")
            }
            NbaError::UnknownEvidence(id) => {
                write!(f, "next-best-action references unknown evidence `{id}`")
            }
        }
    }
}

impl std::error::Error for NbaError {}

/// Everything the deterministic rule needs. Rejection and boundary status
/// are derived from the cited ledger records, never from unchecked booleans
/// or direct message-text inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NbaInput {
    pub stage: Stage,
    /// Evidence ids backing this decision; reasons cite them (rule 7).
    pub evidence_ids: Vec<String>,
    /// Candidates proposed by the prompt layer. This rule never invents reply
    /// text and suppresses all candidates after a boundary or rejection.
    pub proposed_candidates: Vec<ReplyCandidate>,
}

/// Baseline stage → action table. Conservative and documented; every entry
/// is overridden by the boundary/rejection precedence in
/// [`next_best_action`].
fn stage_action(stage: Stage) -> NextBestAction {
    match stage {
        Stage::Unknown => NextBestAction::Wait,
        Stage::Acquaintance => NextBestAction::Continue,
        Stage::ActiveChat => NextBestAction::Change,
        Stage::Familiar => NextBestAction::Continue,
        Stage::Warming => NextBestAction::Continue,
        Stage::Flirting => NextBestAction::Flirt,
        Stage::InvitationReady => NextBestAction::Invite,
        Stage::DatePlanned => NextBestAction::Confirm,
        Stage::Dated => NextBestAction::FollowUp,
        Stage::PostDate => NextBestAction::FollowUp,
    }
}

fn stage_reason(stage: Stage, action: NextBestAction) -> String {
    format!("stage {stage} maps to baseline action {action}")
}

/// Build the reason string, always appending the cited evidence ids so each
/// conclusion stays traceable (AGENTS.md rule 7). Callers validate non-empty
/// ids first.
fn reason_with_evidence(base: &str, evidence_ids: &[String]) -> String {
    format!("{base} (tracked evidence: {})", evidence_ids.join(", "))
}

/// Deterministic precedence:
/// 1. cited `ExplicitRejection` evidence ⇒ [`NextBestAction::Stop`],
///    regardless of stage;
/// 2. else cited `Boundary` evidence ⇒ [`NextBestAction::CoolDown`],
///    regardless of stage (so `InvitationReady` + a boundary must NOT yield
///    `Invite`);
/// 3. else the baseline [`stage_action`] table.
///
/// Every cited id must still exist in the ledger. STOP and COOL_DOWN emit no
/// reply candidates: tone labels alone cannot prove that generated text is
/// safe after a rejection or boundary.
pub fn next_best_action(input: &NbaInput, ledger: &Ledger) -> Result<Advice, NbaError> {
    if input.evidence_ids.is_empty() {
        return Err(NbaError::NoEvidenceIds);
    }
    let mut explicit_rejection = false;
    let mut boundary_signal = false;
    for id in &input.evidence_ids {
        let evidence = ledger
            .get_evidence(id)
            .ok_or_else(|| NbaError::UnknownEvidence(id.clone()))?;
        match evidence.fact_type {
            FactType::ExplicitRejection => {
                explicit_rejection = true;
                boundary_signal = true;
            }
            FactType::Boundary => boundary_signal = true,
            _ => {}
        }
    }

    let action = if explicit_rejection {
        NextBestAction::Stop
    } else if boundary_signal {
        NextBestAction::CoolDown
    } else {
        stage_action(input.stage)
    };

    let base_reason = match action {
        NextBestAction::Stop => {
            "explicit rejection detected; stop all outreach immediately".to_string()
        }
        NextBestAction::CoolDown => {
            "boundary signal present; pause advancement and give space".to_string()
        }
        _ => stage_reason(input.stage, action),
    };

    let reply_candidates = match action {
        NextBestAction::Stop | NextBestAction::CoolDown => Vec::new(),
        _ => input.proposed_candidates.clone(),
    };

    let avoid = match action {
        NextBestAction::Stop => vec![
            "不要继续发送消息或追问原因".to_string(),
            "不要邀约、推进或施压".to_string(),
            "不要重新解读对方的拒绝为暗示".to_string(),
        ],
        NextBestAction::CoolDown => vec![
            "不要推进关系或发出邀约".to_string(),
            "不要高频消息施压".to_string(),
            "给对方空间，等待对方主动".to_string(),
        ],
        _ => vec!["不编造共同经历".to_string()],
    };

    Ok(Advice {
        action,
        reason: reason_with_evidence(&base_reason, &input.evidence_ids),
        evidence_ids: input.evidence_ids.clone(),
        avoid,
        reply_candidates,
    })
}
