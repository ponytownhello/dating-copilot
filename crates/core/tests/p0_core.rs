//! P0 acceptance tests for the deterministic core.
//!
//! These are integration tests exercising the public API exactly as
//! docs/TEST_PLAN.md ("Unit: CanonicalMessage, dedupe, state transition,
//! evidence ledger") and the AGENTS.md rules require.

use dating_core::canonical::{
    compute_dedupe_key, CanonicalError, CanonicalMessage, DedupeIndex, Direction, MessageType,
};
use dating_core::evidence::{Evidence, FactType, Groundedness, Inference, Ledger, LedgerError};
use dating_core::nba::{NbaInput, NextBestAction, ReplyCandidate, ReplyStyle};
use dating_core::state::{adjudicate, AdjudicationError, Proposal, Stage, Trend};

fn message(content: &str, direction: Direction, timestamp_millis: i64) -> CanonicalMessage {
    CanonicalMessage {
        id: format!("m-{content}-{timestamp_millis}"),
        platform: "soul".to_string(),
        conversation_id: "conv-1".to_string(),
        participant_id: "p-1".to_string(),
        direction,
        timestamp_millis,
        message_type: MessageType::Text,
        content: content.to_string(),
        source_ref: "adapter:soul".to_string(),
        dedupe_key: String::new(),
    }
    .finish()
    .expect("valid sample message")
}

fn evidence(id: &str, fact_type: FactType, confidence: u8) -> Evidence {
    Evidence {
        id: id.to_string(),
        fact: format!("fact for {id}"),
        fact_type,
        confidence,
        source_message_ids: vec![format!("msg-{id}")],
    }
}

fn inference(id: &str, supporting: &[&str], counter: &[&str]) -> Inference {
    Inference {
        id: id.to_string(),
        claim: "the other party is warming up".to_string(),
        supporting_evidence_ids: supporting.iter().map(|s| s.to_string()).collect(),
        counter_evidence_ids: counter.iter().map(|s| s.to_string()).collect(),
        unknowns: Vec::new(),
    }
}

fn proposal(from: Stage, to: Stage, trend: Trend, evidence_ids: &[&str]) -> Proposal {
    Proposal {
        from,
        to,
        trend,
        reason: "test proposal".to_string(),
        evidence_ids: evidence_ids.iter().map(|s| s.to_string()).collect(),
    }
}

fn has_ids<'a>(ids: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
    move |id: &str| ids.contains(&id)
}

fn candidate(style: ReplyStyle, text: &str) -> ReplyCandidate {
    ReplyCandidate {
        style,
        text: text.to_string(),
    }
}

// ---- canonical / dedupe ----------------------------------------------------

#[test]
fn dedupe_key_is_deterministic_hex_and_field_sensitive() {
    let key = compute_dedupe_key("soul", "c1", "p1", Direction::Inbound, 100, "hi");
    let again = compute_dedupe_key("soul", "c1", "p1", Direction::Inbound, 100, "hi");
    assert_eq!(key, again);
    assert_eq!(key.len(), 16);
    assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    // Every tuple component changes the key.
    assert_ne!(
        key,
        compute_dedupe_key("xhs", "c1", "p1", Direction::Inbound, 100, "hi")
    );
    assert_ne!(
        key,
        compute_dedupe_key("soul", "c2", "p1", Direction::Inbound, 100, "hi")
    );
    assert_ne!(
        key,
        compute_dedupe_key("soul", "c1", "p2", Direction::Inbound, 100, "hi")
    );
    assert_ne!(
        key,
        compute_dedupe_key("soul", "c1", "p1", Direction::Outbound, 100, "hi")
    );
    assert_ne!(
        key,
        compute_dedupe_key("soul", "c1", "p1", Direction::Inbound, 101, "hi")
    );
    assert_ne!(
        key,
        compute_dedupe_key("soul", "c1", "p1", Direction::Inbound, 100, "ho")
    );
}

#[test]
fn dedupe_index_rejects_exact_duplicate_and_accepts_changed_message() {
    let mut index = DedupeIndex::new();
    let first = message("same text", Direction::Inbound, 10);
    assert_eq!(index.insert(&first), Ok(true), "first insert is new");
    assert_eq!(
        index.insert(&first),
        Ok(false),
        "exact duplicate is not new"
    );
    let changed = message("changed text", Direction::Inbound, 10);
    assert_eq!(index.insert(&changed), Ok(true));
    assert_eq!(index.len(), 2);
    assert!(!index.is_empty());
}

#[test]
fn validation_rejects_blank_content_and_tampered_dedupe_key() {
    let blank_content = CanonicalMessage {
        content: "".to_string(),
        ..message("ok", Direction::Inbound, 5)
    };
    assert_eq!(
        blank_content.validate(),
        Err(CanonicalError::BlankField("content"))
    );
    let whitespace_content = CanonicalMessage {
        content: "   ".to_string(),
        ..message("ok", Direction::Inbound, 5)
    };
    assert_eq!(
        whitespace_content.validate(),
        Err(CanonicalError::BlankField("content"))
    );
    let tampered = CanonicalMessage {
        dedupe_key: "0000000000000000".to_string(),
        ..message("ok", Direction::Inbound, 5)
    };
    assert_eq!(tampered.validate(), Err(CanonicalError::DedupeKeyMismatch));
}

#[test]
fn validation_rejects_blank_ids_and_nonpositive_timestamp() {
    let blank_id = CanonicalMessage {
        id: "  ".to_string(),
        ..message("ok", Direction::Inbound, 5)
    };
    assert_eq!(blank_id.validate(), Err(CanonicalError::BlankField("id")));
    let blank_conversation = CanonicalMessage {
        conversation_id: String::new(),
        ..message("ok", Direction::Inbound, 5)
    };
    assert_eq!(
        blank_conversation.validate(),
        Err(CanonicalError::BlankField("conversation_id"))
    );
    let zero_ts = CanonicalMessage {
        timestamp_millis: 0,
        ..message("ok", Direction::Inbound, 5)
    };
    assert_eq!(
        zero_ts.validate(),
        Err(CanonicalError::NonPositiveTimestamp(0))
    );
}

#[test]
fn dedupe_index_refuses_invalid_message_before_indexing() {
    let mut index = DedupeIndex::new();
    let tampered = CanonicalMessage {
        dedupe_key: "ffffffffffffffff".to_string(),
        ..message("ok", Direction::Inbound, 5)
    };
    assert_eq!(
        index.insert(&tampered),
        Err(CanonicalError::DedupeKeyMismatch)
    );
    assert_eq!(index.len(), 0, "invalid message must never be indexed");
}

// ---- evidence ledger -------------------------------------------------------

#[test]
fn sourceless_evidence_is_rejected_and_not_stored() {
    let mut ledger = Ledger::new();
    let orphan = Evidence {
        id: "x".to_string(),
        fact: "orphan claim".to_string(),
        fact_type: FactType::Observable,
        confidence: 50,
        source_message_ids: Vec::new(),
    };
    assert_eq!(
        ledger.add_evidence(orphan),
        Err(LedgerError::EmptySourceMessageIds("x".to_string()))
    );
    assert!(ledger.is_empty());
}

#[test]
fn sensitive_speculation_fact_is_rejected() {
    let mut ledger = Ledger::new();
    let bad = evidence("s1", FactType::SensitiveSpeculation, 99);
    assert_eq!(
        ledger.add_evidence(bad),
        Err(LedgerError::SensitiveSpeculationRejected("s1".to_string()))
    );
    assert!(!ledger.has_evidence("s1"));
}

#[test]
fn confidence_above_100_is_rejected_and_bounds_accepted() {
    let mut ledger = Ledger::new();
    // `confidence` is a u8, so 101..=255 are the reachable over-range inputs.
    assert_eq!(
        ledger.add_evidence(evidence("hi", FactType::Stated, 101)),
        Err(LedgerError::ConfidenceTooHigh {
            id: "hi".to_string(),
            confidence: 101,
        })
    );
    assert!(ledger
        .add_evidence(evidence("lo", FactType::Stated, 0))
        .is_ok());
    assert!(ledger
        .add_evidence(evidence("max", FactType::Stated, 100))
        .is_ok());
    assert_eq!(ledger.len(), 2);
}

#[test]
fn evidence_less_inference_is_rejected() {
    let mut ledger = Ledger::new();
    assert_eq!(
        ledger.add_inference(inference("i0", &[], &[])),
        Err(LedgerError::EmptySupportingEvidenceIds("i0".to_string()))
    );
}

#[test]
fn unknown_evidence_citation_is_rejected_and_named() {
    let mut ledger = Ledger::new();
    ledger
        .add_evidence(evidence("e1", FactType::Behavioral, 70))
        .unwrap();
    assert_eq!(
        ledger.add_inference(inference("i1", &["e1", "ghost"], &[])),
        Err(LedgerError::UnknownEvidenceReference {
            inference_id: "i1".to_string(),
            evidence_id: "ghost".to_string(),
        })
    );
    // Counter-evidence citations are checked the same way.
    assert_eq!(
        ledger.add_inference(inference("i2", &["e1"], &["phantom"])),
        Err(LedgerError::UnknownEvidenceReference {
            inference_id: "i2".to_string(),
            evidence_id: "phantom".to_string(),
        })
    );
    assert!(ledger.get_inference("i1").is_none());
    assert!(ledger.get_inference("i2").is_none());
}

#[test]
fn ground_check_reports_grounded_partial_and_unsupported() {
    let mut ledger = Ledger::new();
    ledger
        .add_evidence(evidence("e1", FactType::Behavioral, 60))
        .unwrap();
    ledger
        .add_evidence(evidence("e2", FactType::Stated, 80))
        .unwrap();
    ledger
        .add_inference(inference("i1", &["e1", "e2"], &[]))
        .unwrap();
    assert_eq!(
        ledger.ground_check("i1"),
        Some(Groundedness::Grounded),
        "all cited ids present"
    );
    assert!(ledger.remove_evidence("e2"));
    assert_eq!(
        ledger.ground_check("i1"),
        Some(Groundedness::PartiallyGrounded),
        "one cited id now missing"
    );
    assert!(ledger.remove_evidence("e1"));
    assert_eq!(
        ledger.ground_check("i1"),
        Some(Groundedness::Unsupported),
        "no supporting evidence remains"
    );
}

#[test]
fn ground_check_never_fabricates_a_verdict_for_unknown_inference() {
    let ledger = Ledger::new();
    assert_eq!(ledger.ground_check("no-such-inference"), None);
}

// ---- state machine adjudication -------------------------------------------

#[test]
fn adjudicate_rejects_evidence_less_and_unknown_id_proposals() {
    let known = ["e1"];
    assert_eq!(
        adjudicate(
            &proposal(Stage::ActiveChat, Stage::Familiar, Trend::Warming, &[]),
            false,
            has_ids(&known)
        ),
        Err(AdjudicationError::NoEvidenceIds)
    );
    assert_eq!(
        adjudicate(
            &proposal(
                Stage::ActiveChat,
                Stage::Familiar,
                Trend::Warming,
                &["ghost"]
            ),
            false,
            has_ids(&known)
        ),
        Err(AdjudicationError::UnknownEvidence("ghost".to_string()))
    );
}

#[test]
fn adjudicate_allows_one_rank_forward_but_rejects_skip_ahead() {
    let known = ["e1"];
    let forward = adjudicate(
        &proposal(Stage::Familiar, Stage::Warming, Trend::Warming, &["e1"]),
        false,
        has_ids(&known),
    )
    .expect("one-rank forward move allowed");
    assert_eq!(forward.stage, Stage::Warming);
    assert_eq!(forward.trend, Trend::Warming);

    assert_eq!(
        adjudicate(
            &proposal(Stage::Familiar, Stage::Flirting, Trend::Warming, &["e1"]),
            false,
            has_ids(&known)
        ),
        Err(AdjudicationError::SkipForward {
            from: Stage::Familiar,
            to: Stage::Flirting,
            distance: 2,
        })
    );
}

#[test]
fn adjudicate_allows_any_backward_move() {
    let known = ["e1"];
    let backward = adjudicate(
        &proposal(
            Stage::InvitationReady,
            Stage::ActiveChat,
            Trend::Cooling,
            &["e1"],
        ),
        false,
        has_ids(&known),
    )
    .expect("backward moves are never limited in distance");
    assert_eq!(backward.stage, Stage::ActiveChat);
    assert_eq!(backward.trend, Trend::Cooling);
}

#[test]
fn boundary_signal_forbids_any_forward_move() {
    let known = ["e1"];
    assert_eq!(
        adjudicate(
            &proposal(Stage::Familiar, Stage::Warming, Trend::Warming, &["e1"]),
            true,
            has_ids(&known)
        ),
        Err(AdjudicationError::BoundaryBlocksForward { to: Stage::Warming })
    );
    // Even the minimal one-rank step is blocked while a boundary is present.
    assert_eq!(
        adjudicate(
            &proposal(
                Stage::Flirting,
                Stage::InvitationReady,
                Trend::Warming,
                &["e1"]
            ),
            true,
            has_ids(&known)
        ),
        Err(AdjudicationError::BoundaryBlocksForward {
            to: Stage::InvitationReady
        })
    );
}

#[test]
fn boundary_trend_is_carried_regardless_of_stage_move() {
    let known = ["e1"];
    let backward = adjudicate(
        &proposal(Stage::Familiar, Stage::ActiveChat, Trend::Boundary, &["e1"]),
        true,
        has_ids(&known),
    )
    .expect("backward move with boundary trend is allowed");
    assert_eq!(backward.trend, Trend::Boundary);
    assert_eq!(backward.stage, Stage::ActiveChat);

    let lateral = adjudicate(
        &proposal(Stage::Familiar, Stage::Familiar, Trend::Boundary, &["e1"]),
        true,
        has_ids(&known),
    )
    .expect("lateral (no-op) move with boundary trend is allowed");
    assert_eq!(lateral.trend, Trend::Boundary);
    assert_eq!(lateral.stage, Stage::Familiar);
}

#[test]
fn stage_rank_ordering_matches_ladder() {
    assert_eq!(Stage::Unknown.rank(), 0);
    assert_eq!(Stage::Acquaintance.rank(), 1);
    assert_eq!(Stage::InvitationReady.rank(), 6);
    assert_eq!(Stage::PostDate.rank(), 9);
    assert!(Stage::Flirting > Stage::Warming);
    assert!(Stage::InvitationReady > Stage::Flirting);
}

// ---- next best action ------------------------------------------------------

#[test]
fn invitation_ready_with_boundary_never_yields_invite() {
    let input = NbaInput {
        stage: Stage::InvitationReady,
        explicit_rejection: false,
        boundary_signal: true,
        evidence_ids: vec!["b1".to_string()],
        proposed_candidates: vec![
            candidate(ReplyStyle::Natural, "好，你先忙"),
            candidate(ReplyStyle::GentleProgress, "那这周末出来喝杯咖啡？"),
        ],
    };
    let advice = dating_core::next_best_action(&input);
    assert_eq!(advice.action, NextBestAction::CoolDown);
    assert!(!advice
        .reply_candidates
        .iter()
        .any(|c| c.style == ReplyStyle::GentleProgress));
    assert_eq!(advice.evidence_ids, vec!["b1".to_string()]);
    assert!(advice.reason.contains("b1"), "reason must cite evidence");
    assert!(!advice.reason.contains('%'), "no percentage anywhere");
    assert!(!advice.avoid.is_empty());
}

#[test]
fn explicit_rejection_yields_stop_and_dominates_everything() {
    let input = NbaInput {
        stage: Stage::InvitationReady,
        explicit_rejection: true,
        boundary_signal: true,
        evidence_ids: vec!["b1".to_string(), "b2".to_string()],
        proposed_candidates: vec![
            candidate(ReplyStyle::Humorous, "开个玩笑缓解一下"),
            candidate(ReplyStyle::GentleProgress, "改天再约？"),
        ],
    };
    let advice = dating_core::next_best_action(&input);
    assert_eq!(advice.action, NextBestAction::Stop);
    assert!(advice
        .reply_candidates
        .iter()
        .all(|c| c.style != ReplyStyle::GentleProgress));
    assert!(advice.reason.contains("b1"));
    assert!(advice.reason.contains("b2"));
}

#[test]
fn normal_invitation_ready_without_boundary_yields_invite_with_evidence() {
    let input = NbaInput {
        stage: Stage::InvitationReady,
        explicit_rejection: false,
        boundary_signal: false,
        evidence_ids: vec!["e1".to_string(), "e2".to_string()],
        proposed_candidates: vec![candidate(
            ReplyStyle::GentleProgress,
            "周六下午有场展览，一起去看？",
        )],
    };
    let advice = dating_core::next_best_action(&input);
    assert_eq!(advice.action, NextBestAction::Invite);
    assert_eq!(
        advice.evidence_ids,
        vec!["e1".to_string(), "e2".to_string()]
    );
    assert!(advice.reason.contains("e1") && advice.reason.contains("e2"));
    assert_eq!(
        advice.reply_candidates.len(),
        1,
        "no filtering without boundary"
    );
}

#[test]
fn boundary_precedence_covers_every_stage() {
    for stage in [
        Stage::Unknown,
        Stage::Acquaintance,
        Stage::ActiveChat,
        Stage::Familiar,
        Stage::Warming,
        Stage::Flirting,
        Stage::InvitationReady,
        Stage::DatePlanned,
        Stage::Dated,
        Stage::PostDate,
    ] {
        let cooldown = dating_core::next_best_action(&NbaInput {
            stage,
            explicit_rejection: false,
            boundary_signal: true,
            evidence_ids: vec!["b1".to_string()],
            proposed_candidates: Vec::new(),
        });
        assert_eq!(cooldown.action, NextBestAction::CoolDown, "stage {stage}");
        let stop = dating_core::next_best_action(&NbaInput {
            stage,
            explicit_rejection: true,
            boundary_signal: false,
            evidence_ids: vec!["b1".to_string()],
            proposed_candidates: Vec::new(),
        });
        assert_eq!(stop.action, NextBestAction::Stop, "stage {stage}");
    }
}
