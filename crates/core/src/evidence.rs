//! Evidence ledger, inferences and grounding checks.
//!
//! Evidence and Inference are kept as separate types (AGENTS.md rule 4). A
//! fact with no source must never enter long-term memory
//! (ARCHITECTURE.md "Evidence Ledger"), and sensitive/speculative attributes
//! must not be asserted from limited chat (AGENTS.md rule 10). Both are
//! encoded as machine gates that fail closed, not as comments.
//!
//! Deterministic and dependency-free: the ledger is backed by ordered maps so
//! every iteration is stable and reproducible (no `HashMap` ordering leaks).

use std::collections::BTreeMap;
use std::fmt;

/// Category of an atomic observation. Only observable, stated, behavioural
/// and boundary facts may be recorded; `SensitiveSpeculation` exists purely
/// so the machine gate can *reject* it (AGENTS.md rule 10: no asserting
/// diseases, trauma, IQ, wealth or personality disorders from limited chat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactType {
    /// Directly observable in a message (text, timing, counts).
    Observable,
    /// Something the other party explicitly stated.
    Stated,
    /// A pattern of behaviour across messages.
    Behavioral,
    /// A boundary or rejection signal (AGENTS.md rule 9).
    Boundary,
    /// A sensitive or unverifiable attribute claim. REJECTED at insertion.
    SensitiveSpeculation,
}

/// A single sourced observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub fact: String,
    pub fact_type: FactType,
    /// Percent-style confidence, valid range 0..=100 (checked at insertion;
    /// the field is a `u8` so only the upper bound is a runtime gate).
    pub confidence: u8,
    pub source_message_ids: Vec<String>,
}

/// A conclusion that must be traceable back to evidence (AGENTS.md rule 7).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inference {
    pub id: String,
    pub claim: String,
    pub supporting_evidence_ids: Vec<String>,
    pub counter_evidence_ids: Vec<String>,
    pub unknowns: Vec<String>,
}

/// Result of grounding a stored inference against the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Groundedness {
    /// Every cited evidence id is present in the ledger.
    Grounded,
    /// Some cited evidence ids are missing from the ledger.
    PartiallyGrounded,
    /// No cited (supporting) evidence is present in the ledger.
    Unsupported,
}

/// Hard errors raised by the ledger. Every variant means "nothing was
/// stored" — the ledger fails closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// Evidence without any source message id (rule 4 / "no source, no
    /// long-term memory").
    EmptySourceMessageIds(String),
    /// Confidence above 100 (the valid range is 0..=100).
    ConfidenceTooHigh { id: String, confidence: u8 },
    /// A `SensitiveSpeculation` fact: rejected by the machine gate that
    /// encodes AGENTS.md rule 10.
    SensitiveSpeculationRejected(String),
    /// An evidence id already exists in the ledger.
    DuplicateEvidenceId(String),
    /// An inference id already exists in the ledger.
    DuplicateInferenceId(String),
    /// An inference cites zero supporting evidence ids.
    EmptySupportingEvidenceIds(String),
    /// An inference cites an evidence id the ledger does not know.
    UnknownEvidenceReference {
        inference_id: String,
        evidence_id: String,
    },
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::EmptySourceMessageIds(id) => {
                write!(f, "evidence `{id}` has no source_message_ids")
            }
            LedgerError::ConfidenceTooHigh { id, confidence } => {
                write!(f, "evidence `{id}` has confidence {confidence} > 100")
            }
            LedgerError::SensitiveSpeculationRejected(id) => {
                write!(
                    f,
                    "evidence `{id}` is a sensitive speculation and must not enter memory"
                )
            }
            LedgerError::DuplicateEvidenceId(id) => write!(f, "duplicate evidence id `{id}`"),
            LedgerError::DuplicateInferenceId(id) => write!(f, "duplicate inference id `{id}`"),
            LedgerError::EmptySupportingEvidenceIds(id) => {
                write!(f, "inference `{id}` cites no supporting evidence")
            }
            LedgerError::UnknownEvidenceReference {
                inference_id,
                evidence_id,
            } => {
                write!(
                    f,
                    "inference `{inference_id}` references unknown evidence `{evidence_id}`"
                )
            }
        }
    }
}

/// In-memory, deterministic evidence + inference store.
///
/// Backed by `BTreeMap`s keyed on id so all iteration is in a stable, sorted
/// order (never the arbitrary order a `HashMap` would expose).
#[derive(Debug, Default)]
pub struct Ledger {
    evidence: BTreeMap<String, Evidence>,
    inferences: BTreeMap<String, Inference>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert evidence, failing closed on sourceless facts, out-of-range
    /// confidence, sensitive speculations and duplicate ids.
    pub fn add_evidence(&mut self, evidence: Evidence) -> Result<(), LedgerError> {
        if evidence.fact_type == FactType::SensitiveSpeculation {
            return Err(LedgerError::SensitiveSpeculationRejected(evidence.id));
        }
        if evidence.source_message_ids.is_empty() {
            return Err(LedgerError::EmptySourceMessageIds(evidence.id));
        }
        if evidence.confidence > 100 {
            return Err(LedgerError::ConfidenceTooHigh {
                id: evidence.id,
                confidence: evidence.confidence,
            });
        }
        if self.evidence.contains_key(&evidence.id) {
            return Err(LedgerError::DuplicateEvidenceId(evidence.id));
        }
        self.evidence.insert(evidence.id.clone(), evidence);
        Ok(())
    }

    /// Register an inference. Fails closed when it cites no supporting
    /// evidence or when any cited id (supporting or counter) is unknown to
    /// the ledger — the error names the offending evidence id.
    pub fn add_inference(&mut self, inference: Inference) -> Result<(), LedgerError> {
        if inference.supporting_evidence_ids.is_empty() {
            return Err(LedgerError::EmptySupportingEvidenceIds(inference.id));
        }
        for evidence_id in inference
            .supporting_evidence_ids
            .iter()
            .chain(inference.counter_evidence_ids.iter())
        {
            if !self.evidence.contains_key(evidence_id) {
                return Err(LedgerError::UnknownEvidenceReference {
                    inference_id: inference.id.clone(),
                    evidence_id: evidence_id.clone(),
                });
            }
        }
        if self.inferences.contains_key(&inference.id) {
            return Err(LedgerError::DuplicateInferenceId(inference.id));
        }
        self.inferences.insert(inference.id.clone(), inference);
        Ok(())
    }

    pub fn has_evidence(&self, id: &str) -> bool {
        self.evidence.contains_key(id)
    }

    pub fn get_evidence(&self, id: &str) -> Option<&Evidence> {
        self.evidence.get(id)
    }

    pub fn get_inference(&self, id: &str) -> Option<&Inference> {
        self.inferences.get(id)
    }

    /// Privacy deletion (ARCHITECTURE.md "Security": contact/conversation
    /// level deletion). Removing evidence can leave previously stored
    /// inferences partially or fully ungrounded; [`Ledger::ground_check`]
    /// reports that instead of pretending it never happened.
    pub fn remove_evidence(&mut self, id: &str) -> bool {
        self.evidence.remove(id).is_some()
    }

    /// Trace a stored conclusion back to the evidence ids it cites
    /// (supporting first, then counter). `None` if the inference is unknown.
    pub fn evidence_ids_for(&self, inference_id: &str) -> Option<Vec<String>> {
        self.inferences.get(inference_id).map(|inf| {
            let mut ids = inf.supporting_evidence_ids.clone();
            ids.extend(inf.counter_evidence_ids.iter().cloned());
            ids
        })
    }

    /// Ground a *stored* inference by id.
    ///
    /// Returns `None` for an inference id the ledger does not know: no
    /// verdict is ever fabricated for an unknown inference. Otherwise:
    /// - [`Groundedness::Grounded`]: at least one supporting id is cited and
    ///   every cited id (supporting and counter) is present.
    /// - [`Groundedness::PartiallyGrounded`]: at least one cited id is still
    ///   present but some cited ids are missing.
    /// - [`Groundedness::Unsupported`]: no supporting evidence is present.
    pub fn ground_check(&self, inference_id: &str) -> Option<Groundedness> {
        let inference = self.inferences.get(inference_id)?;
        let cited: Vec<&String> = inference
            .supporting_evidence_ids
            .iter()
            .chain(inference.counter_evidence_ids.iter())
            .collect();
        let known_cited = cited.iter().filter(|id| self.has_evidence(id)).count();
        let support_known = inference
            .supporting_evidence_ids
            .iter()
            .any(|id| self.has_evidence(id));
        if !support_known {
            Some(Groundedness::Unsupported)
        } else if known_cited == cited.len() {
            Some(Groundedness::Grounded)
        } else {
            Some(Groundedness::PartiallyGrounded)
        }
    }

    /// Number of stored evidence entries.
    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }

    /// All evidence ids in stable sorted order.
    pub fn evidence_ids(&self) -> Vec<String> {
        self.evidence.keys().cloned().collect()
    }
}
