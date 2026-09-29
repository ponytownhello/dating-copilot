//! Canonical message schema and content-derived deduplication.
//!
//! Every inbound source (screenshot/OCR, browser adapter, notification) is
//! normalized into [`CanonicalMessage`] before anything else touches it
//! (AGENTS.md rule 2, ARCHITECTURE.md "CanonicalMessage"). The struct carries
//! the minimum fields documented in ARCHITECTURE.md.

use std::collections::HashSet;
use std::fmt;

/// Who produced the message, relative to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Received from the other participant.
    Inbound,
    /// Sent by the user.
    Outbound,
}

impl Direction {
    /// Stable, lowercase tag used when computing the dedup key.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Inbound => "inbound",
            Direction::Outbound => "outbound",
        }
    }
}

/// Modalities a canonical message can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    Text,
    Image,
    Voice,
    System,
    Unknown,
}

/// A single normalized message.
///
/// The ARCHITECTURE.md field named `type` is stored here as `message_type`
/// because `type` is a reserved Rust keyword. `timestamp_millis` is the
/// documented `timestamp` field, fixed to milliseconds so dedup keys are
/// unambiguous.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalMessage {
    pub id: String,
    /// Plain `String` by design: adapters fill it ("soul", "xiaohongshu", ...).
    pub platform: String,
    pub conversation_id: String,
    pub participant_id: String,
    pub direction: Direction,
    pub timestamp_millis: i64,
    pub message_type: MessageType,
    pub content: String,
    pub source_ref: String,
    /// Content-derived key. Never trusted when supplied by a caller;
    /// [`compute_dedupe_key`] is the only authority (see
    /// [`CanonicalMessage::recomputed_dedupe_key`] and
    /// [`CanonicalMessage::finish`]).
    pub dedupe_key: String,
}

impl Default for CanonicalMessage {
    fn default() -> Self {
        CanonicalMessage {
            id: String::new(),
            platform: String::new(),
            conversation_id: String::new(),
            participant_id: String::new(),
            direction: Direction::Inbound,
            timestamp_millis: 0,
            message_type: MessageType::Unknown,
            content: String::new(),
            source_ref: String::new(),
            dedupe_key: String::new(),
        }
    }
}

/// Reasons a canonical message fails validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalError {
    /// The named required field is empty or whitespace-only.
    BlankField(&'static str),
    /// `timestamp_millis` must be strictly positive.
    NonPositiveTimestamp(i64),
    /// The stored `dedupe_key` does not match the recomputed one.
    DedupeKeyMismatch,
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanonicalError::BlankField(name) => {
                write!(f, "required field `{name}` is blank")
            }
            CanonicalError::NonPositiveTimestamp(ts) => {
                write!(f, "timestamp_millis must be positive, got {ts}")
            }
            CanonicalError::DedupeKeyMismatch => {
                write!(f, "dedupe_key does not match the recomputed value")
            }
        }
    }
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Fold raw bytes into an FNV-1a 64-bit state.
fn fold(hash: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *hash ^= byte as u64;
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

/// Fold a length-prefixed field so tuple boundaries can never blur together.
fn fold_field(hash: &mut u64, bytes: &[u8]) {
    fold(hash, &(bytes.len() as u64).to_le_bytes());
    fold(hash, bytes);
}

/// Deterministic deduplication identity: FNV-1a 64-bit, rendered as exactly
/// 16 lowercase hex characters, seeded over the tuple
/// (platform, conversation_id, participant_id, direction, timestamp_millis,
/// content).
///
/// This is a deduplication identity only and carries no security meaning.
pub fn compute_dedupe_key(
    platform: &str,
    conversation_id: &str,
    participant_id: &str,
    direction: Direction,
    timestamp_millis: i64,
    content: &str,
) -> String {
    let mut hash = FNV_OFFSET_BASIS;
    fold_field(&mut hash, platform.as_bytes());
    fold_field(&mut hash, conversation_id.as_bytes());
    fold_field(&mut hash, participant_id.as_bytes());
    fold_field(&mut hash, direction.as_str().as_bytes());
    fold(&mut hash, &timestamp_millis.to_le_bytes());
    fold_field(&mut hash, content.as_bytes());
    format!("{hash:016x}")
}

impl CanonicalMessage {
    /// Recompute the key from content; the only sanctioned source of truth.
    pub fn recomputed_dedupe_key(&self) -> String {
        compute_dedupe_key(
            &self.platform,
            &self.conversation_id,
            &self.participant_id,
            self.direction,
            self.timestamp_millis,
            &self.content,
        )
    }

    /// Validate the message. Checks blank ids/content and the timestamp
    /// first, then that the stored dedup key matches the recomputed one.
    pub fn validate(&self) -> Result<(), CanonicalError> {
        if self.id.trim().is_empty() {
            return Err(CanonicalError::BlankField("id"));
        }
        if self.platform.trim().is_empty() {
            return Err(CanonicalError::BlankField("platform"));
        }
        if self.conversation_id.trim().is_empty() {
            return Err(CanonicalError::BlankField("conversation_id"));
        }
        if self.participant_id.trim().is_empty() {
            return Err(CanonicalError::BlankField("participant_id"));
        }
        if self.content.trim().is_empty() {
            return Err(CanonicalError::BlankField("content"));
        }
        if self.source_ref.trim().is_empty() {
            return Err(CanonicalError::BlankField("source_ref"));
        }
        if self.timestamp_millis <= 0 {
            return Err(CanonicalError::NonPositiveTimestamp(self.timestamp_millis));
        }
        if self.dedupe_key != self.recomputed_dedupe_key() {
            return Err(CanonicalError::DedupeKeyMismatch);
        }
        Ok(())
    }

    /// Set the dedup key from content (never trusting a caller-supplied value)
    /// and validate the result.
    pub fn finish(mut self) -> Result<Self, CanonicalError> {
        self.dedupe_key = self.recomputed_dedupe_key();
        self.validate()?;
        Ok(self)
    }
}

/// Tracks seen dedup keys; a duplicate insert is reported, not stored.
#[derive(Debug, Default)]
pub struct DedupeIndex {
    seen: HashSet<String>,
}

impl DedupeIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a message using its *recomputed* key (any stored key value on
    /// the message is re-derived and must pass validation first). Returns
    /// `true` if the message was new, `false` if an identical message was
    /// already indexed. A message that fails [`CanonicalMessage::validate`]
    /// is rejected before it can ever be indexed.
    pub fn insert(&mut self, message: &CanonicalMessage) -> Result<bool, CanonicalError> {
        message.validate()?;
        let key = message.recomputed_dedupe_key();
        Ok(self.seen.insert(key))
    }

    /// Return `true` if an already-computed key has been seen.
    pub fn contains_key(&self, key: &str) -> bool {
        self.seen.contains(key)
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}
