//! Prompt Registry: versioned prompt specs, deterministic rendering and schema validation.
//!
//! This is the P0 Prompt Harness listed by docs/ROADMAP.md. A prompt is a
//! discrete [`PromptSpec`] recording the identity fields docs/PROMPT_DESIGN.md
//! mandates (id / version / schema / model-family / eval-set), registered in a
//! [`Registry`] keyed by id (AGENTS.md rule 6: every prompt/skill carries a
//! version, a schema and an eval-set).
//!
//! The registry also encodes the anti-super-prompt rule from
//! docs/PROMPT_DESIGN.md ("禁止维护不断膨胀的超级 Prompt"): a new spec for a
//! role must carry a strictly higher [`Version`] than every spec already
//! registered for that role, so a prompt can only ever be superseded by an
//! explicitly newer version rather than silently grown.
//!
//! Deterministic and dependency-free: [`Registry::render`] substitutes
//! `{{name}}` placeholders and leaves literal text untouched, and
//! [`Registry::validate_output`] accepts only an output that declares exactly
//! the keys of its schema (AGENTS.md rule 3). No I/O, no randomness, no
//! wall-clock reads.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The seven prompt roles defined by docs/PROMPT_DESIGN.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromptRole {
    /// Global copilot persona and hard rules.
    System,
    /// Turn raw canonical messages into sourced evidence.
    Evidence,
    /// Describe only what a screenshot shows.
    Vision,
    /// Strong reasoning and multi-hypothesis interpretation.
    Reasoning,
    /// Stage/trend relationship conclusions.
    Relationship,
    /// Next-best-action and reply candidates.
    Action,
    /// Critic/ground check that binds conclusions to evidence.
    Review,
}

impl PromptRole {
    /// Stable uppercase tag.
    pub fn as_str(self) -> &'static str {
        match self {
            PromptRole::System => "SYSTEM",
            PromptRole::Evidence => "EVIDENCE",
            PromptRole::Vision => "VISION",
            PromptRole::Reasoning => "REASONING",
            PromptRole::Relationship => "RELATIONSHIP",
            PromptRole::Action => "ACTION",
            PromptRole::Review => "REVIEW",
        }
    }
}

impl fmt::Display for PromptRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A `major.minor` prompt version. `Ord` compares major then minor, so a
/// strictly increasing version per role is well defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

impl Version {
    /// Parse a `major.minor` string. Fails when the text is not exactly two
    /// dot-separated non-negative integers.
    pub fn parse(text: &str) -> Result<Version, PromptError> {
        let mut parts = text.split('.');
        let major = parts
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| PromptError::MalformedVersion(text.to_string()))?;
        let minor = parts
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| PromptError::MalformedVersion(text.to_string()))?;
        if parts.next().is_some() {
            return Err(PromptError::MalformedVersion(text.to_string()));
        }
        Ok(Version { major, minor })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A single, versioned prompt definition.
///
/// The identity fields mirror docs/PROMPT_DESIGN.md; `output_keys` is the set
/// of keys `schema` declares and `template` is the body rendered by
/// [`Registry::render`]. Both let the registry validate an output and render a
/// prompt without any external lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSpec {
    /// Unique prompt id.
    pub id: String,
    pub version: Version,
    pub role: PromptRole,
    /// Output schema identifier (the recorded "schema" field).
    pub schema: String,
    /// The exact keys [`PromptSpec::schema`] declares; used by
    /// [`Registry::validate_output`].
    pub output_keys: Vec<String>,
    /// Model family this prompt targets (never trusted to judge text safety).
    pub model_family: String,
    /// Eval set this prompt must pass (AGENTS.md rule 6).
    pub eval_set: String,
    /// The prompt body, containing `{{name}}` placeholders.
    pub template: String,
}

impl PromptSpec {
    /// Checked constructor. Fails closed on a blank id, schema, model family
    /// or eval set (AGENTS.md rule 6). The template may be empty; output keys
    /// are stored in the given order.
    // Every parameter is a mandatory identity field of the recorded prompt; a
    // builder would only add a second way to leave one unset.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        version: Version,
        role: PromptRole,
        schema: &str,
        output_keys: &[&str],
        model_family: &str,
        eval_set: &str,
        template: &str,
    ) -> Result<PromptSpec, PromptError> {
        if id.trim().is_empty() {
            return Err(PromptError::EmptyField("id"));
        }
        if schema.trim().is_empty() {
            return Err(PromptError::EmptyField("schema"));
        }
        if model_family.trim().is_empty() {
            return Err(PromptError::EmptyField("model_family"));
        }
        if eval_set.trim().is_empty() {
            return Err(PromptError::EmptyField("eval_set"));
        }
        let mut unique_output_keys = BTreeSet::new();
        for key in output_keys {
            if !unique_output_keys.insert(*key) {
                return Err(PromptError::DuplicateOutputKey((*key).to_string()));
            }
        }
        Ok(PromptSpec {
            id: id.to_string(),
            version,
            role,
            schema: schema.to_string(),
            output_keys: output_keys.iter().map(|key| key.to_string()).collect(),
            model_family: model_family.to_string(),
            eval_set: eval_set.to_string(),
            template: template.to_string(),
        })
    }
}

/// Hard errors raised by the prompt layer. Every variant means nothing was
/// stored, rendered or accepted — the registry fails closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptError {
    /// A required [`PromptSpec`] field is blank.
    EmptyField(&'static str),
    /// A prompt id is already registered.
    DuplicateId(String),
    /// A prompt version is not strictly greater than the highest version
    /// already registered for the same role (the anti-super-prompt rule).
    VersionNotIncreasing {
        role: PromptRole,
        highest: Version,
        candidate: Version,
    },
    /// The version text is not `major.minor`.
    MalformedVersion(String),
    /// A `{{name}}` placeholder has no value in `vars`.
    MissingVariable(String),
    /// A placeholder is malformed (empty name or a stray brace) or unclosed.
    MalformedPlaceholder(String),
    /// The output omits a key its schema declares.
    MissingOutputKey(String),
    /// The output declares a key its schema does not.
    ExtraOutputKey(String),
    /// The same output key appears more than once in a schema or result.
    DuplicateOutputKey(String),
    /// No prompt is registered under the given id.
    UnknownPrompt(String),
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromptError::EmptyField(field) => write!(f, "prompt field `{field}` is blank"),
            PromptError::DuplicateId(id) => write!(f, "prompt id `{id}` is already registered"),
            PromptError::VersionNotIncreasing {
                role,
                highest,
                candidate,
            } => {
                write!(
                    f,
                    "prompt version {candidate} for role {role} must be strictly greater than the highest registered version {highest}"
                )
            }
            PromptError::MalformedVersion(text) => {
                write!(f, "malformed version `{text}`, expected `major.minor`")
            }
            PromptError::MissingVariable(name) => {
                write!(f, "placeholder `{{{{{name}}}}}` has no value")
            }
            PromptError::MalformedPlaceholder(text) => {
                write!(f, "malformed or unclosed placeholder `{text}`")
            }
            PromptError::MissingOutputKey(key) => {
                write!(f, "output is missing declared key `{key}`")
            }
            PromptError::ExtraOutputKey(key) => {
                write!(f, "output declares undeclared key `{key}`")
            }
            PromptError::DuplicateOutputKey(key) => {
                write!(f, "output key `{key}` appears more than once")
            }
            PromptError::UnknownPrompt(id) => write!(f, "no prompt registered for id `{id}`"),
        }
    }
}

impl std::error::Error for PromptError {}

/// In-memory, deterministic set of registered prompt specs.
///
/// Backed by `BTreeMap`s keyed on id (and role) so iteration and the
/// per-role version gate are stable and reproducible (never `HashMap`
/// ordering).
#[derive(Debug, Default)]
pub struct Registry {
    specs: BTreeMap<String, PromptSpec>,
    highest_by_role: BTreeMap<PromptRole, Version>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a spec. Fails closed on a duplicate id (see
    /// [`PromptError::DuplicateId`]) and on a version that is not strictly
    /// greater than the highest version already registered for the spec's
    /// role (see [`PromptError::VersionNotIncreasing`]): the anti-super-prompt
    /// rule from docs/PROMPT_DESIGN.md. Nothing is stored when either error is
    /// returned.
    pub fn register(&mut self, spec: PromptSpec) -> Result<(), PromptError> {
        if self.specs.contains_key(&spec.id) {
            return Err(PromptError::DuplicateId(spec.id));
        }
        if let Some(highest) = self.highest_by_role.get(&spec.role).copied() {
            if spec.version <= highest {
                return Err(PromptError::VersionNotIncreasing {
                    role: spec.role,
                    highest,
                    candidate: spec.version,
                });
            }
        }
        self.highest_by_role.insert(spec.role, spec.version);
        self.specs.insert(spec.id.clone(), spec);
        Ok(())
    }

    /// Look up a registered spec by id.
    pub fn get(&self, id: &str) -> Option<&PromptSpec> {
        self.specs.get(id)
    }

    /// Substitute every `{{name}}` placeholder in the registered template with
    /// its value in `vars`, deterministically and in a single pass. Literal
    /// text is copied untouched; a single `{` outside a `{{...}}` placeholder
    /// stays literal.
    ///
    /// Fails with [`PromptError::MissingVariable`] when a placeholder name has
    /// no value, [`PromptError::MalformedPlaceholder`] on an unclosed or
    /// malformed placeholder, and [`PromptError::UnknownPrompt`] for an id the
    /// registry does not know.
    pub fn render(&self, id: &str, vars: &[(&str, &str)]) -> Result<String, PromptError> {
        let spec = self
            .specs
            .get(id)
            .ok_or_else(|| PromptError::UnknownPrompt(id.to_string()))?;
        let chars: Vec<char> = spec.template.chars().collect();
        let total = chars.len();
        let mut rendered = String::new();
        let mut cursor = 0usize;
        while cursor < total {
            if chars[cursor] == '{' && cursor + 1 < total && chars[cursor + 1] == '{' {
                let open = cursor;
                let name_start = cursor + 2;
                let mut closed = false;
                let mut index = cursor + 2;
                while index < total {
                    if chars[index] == '{' {
                        let fragment: String = chars[open..=index].iter().collect();
                        return Err(PromptError::MalformedPlaceholder(fragment));
                    }
                    if chars[index] == '}' {
                        if index + 1 < total && chars[index + 1] == '}' {
                            closed = true;
                            break;
                        }
                        let fragment: String = chars[open..=index].iter().collect();
                        return Err(PromptError::MalformedPlaceholder(fragment));
                    }
                    index += 1;
                }
                if !closed {
                    let fragment: String = chars[open..].iter().collect();
                    return Err(PromptError::MalformedPlaceholder(fragment));
                }
                let name_end = index;
                if name_start == name_end {
                    let fragment: String = chars[open..name_end + 2].iter().collect();
                    return Err(PromptError::MalformedPlaceholder(fragment));
                }
                let name: String = chars[name_start..name_end].iter().collect();
                match vars.iter().find(|(key, _)| *key == name.as_str()) {
                    Some((_, value)) => rendered.push_str(value),
                    None => return Err(PromptError::MissingVariable(name)),
                }
                cursor = name_end + 2;
            } else {
                rendered.push(chars[cursor]);
                cursor += 1;
            }
        }
        Ok(rendered)
    }

    /// Validate that an output declares exactly the keys of the registered
    /// spec's schema (AGENTS.md rule 3). Rejects duplicate keys before checking
    /// for the first missing declared key ([`PromptError::MissingOutputKey`])
    /// or extra key ([`PromptError::ExtraOutputKey`]).
    pub fn validate_output(&self, id: &str, keys: &[&str]) -> Result<(), PromptError> {
        let spec = self
            .specs
            .get(id)
            .ok_or_else(|| PromptError::UnknownPrompt(id.to_string()))?;
        let mut seen = BTreeSet::new();
        for key in keys {
            if !seen.insert(*key) {
                return Err(PromptError::DuplicateOutputKey((*key).to_string()));
            }
        }
        for declared in &spec.output_keys {
            if !keys.contains(&declared.as_str()) {
                return Err(PromptError::MissingOutputKey(declared.clone()));
            }
        }
        for key in keys {
            if !spec.output_keys.iter().any(|declared| declared == key) {
                return Err(PromptError::ExtraOutputKey((*key).to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, role: PromptRole, version: &str, keys: &[&str]) -> PromptSpec {
        PromptSpec::new(
            id,
            Version::parse(version).expect("valid version"),
            role,
            "schema",
            keys,
            "model-family",
            "eval-set",
            "template",
        )
        .expect("valid spec")
    }

    #[test]
    fn register_then_get_round_trip() {
        let mut registry = Registry::new();
        let built = spec("review-1", PromptRole::Review, "1.0", &["a"]);
        registry
            .register(built.clone())
            .expect("first registration");
        assert_eq!(registry.get("review-1"), Some(&built));
    }

    #[test]
    fn empty_field_rejected_by_constructor() {
        let blank_id = PromptSpec::new(
            "",
            Version::parse("1.0").expect("valid version"),
            PromptRole::System,
            "schema",
            &["a"],
            "model-family",
            "eval-set",
            "template",
        );
        assert_eq!(
            blank_id.unwrap_err(),
            PromptError::EmptyField("id"),
            "a blank id must be rejected"
        );

        let blank_schema = PromptSpec::new(
            "id",
            Version::parse("1.0").expect("valid version"),
            PromptRole::System,
            " ",
            &["a"],
            "model-family",
            "eval-set",
            "template",
        );
        assert_eq!(
            blank_schema.unwrap_err(),
            PromptError::EmptyField("schema"),
            "a blank schema must be rejected"
        );

        let blank_model = PromptSpec::new(
            "id",
            Version::parse("1.0").expect("valid version"),
            PromptRole::System,
            "schema",
            &["a"],
            "",
            "eval-set",
            "template",
        );
        assert_eq!(
            blank_model.unwrap_err(),
            PromptError::EmptyField("model_family"),
            "a blank model family must be rejected"
        );

        let blank_eval = PromptSpec::new(
            "id",
            Version::parse("1.0").expect("valid version"),
            PromptRole::System,
            "schema",
            &["a"],
            "model-family",
            "  ",
            "template",
        );
        assert_eq!(
            blank_eval.unwrap_err(),
            PromptError::EmptyField("eval_set"),
            "a blank eval set must be rejected"
        );
    }

    #[test]
    fn duplicate_id_rejected() {
        let mut registry = Registry::new();
        registry
            .register(spec("a", PromptRole::System, "1.0", &["k"]))
            .expect("first registration");
        let second = registry.register(spec("a", PromptRole::System, "9.9", &["k"]));
        assert_eq!(
            second.unwrap_err(),
            PromptError::DuplicateId("a".to_string())
        );
    }

    #[test]
    fn non_increasing_version_rejected() {
        let mut registry = Registry::new();
        registry
            .register(spec("v21", PromptRole::Reasoning, "2.1", &["k"]))
            .expect("first registration at 2.1");
        let lower = registry.register(spec("v20", PromptRole::Reasoning, "2.0", &["k"]));
        assert!(
            matches!(lower, Err(PromptError::VersionNotIncreasing { .. })),
            "a lower version for the same role must be rejected"
        );
    }

    #[test]
    fn increasing_version_accepted() {
        let mut registry = Registry::new();
        registry
            .register(spec("v10", PromptRole::Vision, "1.0", &["k"]))
            .expect("first registration at 1.0");
        registry
            .register(spec("v11", PromptRole::Vision, "1.1", &["k"]))
            .expect("a legal increase to 1.1 must be accepted");
        registry
            .register(spec("v20", PromptRole::Vision, "2.0", &["k"]))
            .expect("a legal increase to 2.0 must be accepted");
        assert!(registry.get("v20").is_some());
    }

    #[test]
    fn every_role_round_trips_through_as_str() {
        let roles = [
            (PromptRole::System, "SYSTEM"),
            (PromptRole::Evidence, "EVIDENCE"),
            (PromptRole::Vision, "VISION"),
            (PromptRole::Reasoning, "REASONING"),
            (PromptRole::Relationship, "RELATIONSHIP"),
            (PromptRole::Action, "ACTION"),
            (PromptRole::Review, "REVIEW"),
        ];
        for (role, tag) in roles {
            assert_eq!(role.as_str(), tag);
            assert_eq!(role.to_string(), tag);
        }
    }

    #[test]
    fn render_replaces_placeholders_and_keeps_literals() {
        let mut registry = Registry::new();
        let built = PromptSpec::new(
            "action",
            Version::parse("1.0").expect("valid version"),
            PromptRole::Action,
            "schema",
            &["k"],
            "model-family",
            "eval-set",
            "Hi {{name}}, you sent {{count}}. a} b{c literal {brace}.",
        )
        .expect("valid spec");
        registry.register(built).expect("registration");
        let rendered = registry
            .render("action", &[("name", "Ada"), ("count", "3")])
            .expect("well-formed template");
        assert_eq!(rendered, "Hi Ada, you sent 3. a} b{c literal {brace}.");
    }

    #[test]
    fn render_missing_variable_is_error() {
        let mut registry = Registry::new();
        let built = PromptSpec::new(
            "action",
            Version::parse("1.0").expect("valid version"),
            PromptRole::Action,
            "schema",
            &["k"],
            "model-family",
            "eval-set",
            "Hello {{name}}!",
        )
        .expect("valid spec");
        registry.register(built).expect("registration");
        assert_eq!(
            registry.render("action", &[]).unwrap_err(),
            PromptError::MissingVariable("name".to_string())
        );
    }

    #[test]
    fn render_malformed_placeholder_is_error() {
        let mut registry = Registry::new();
        for (index, body) in ["{{ name", "{{}}", "{{a}b}}"].into_iter().enumerate() {
            let built = PromptSpec::new(
                &format!("malformed-{index}"),
                // The role repeats across the loop, so each spec must be a strictly
                // higher version of the one before it, as the registry demands.
                Version::parse(&format!("1.{index}")).expect("valid version"),
                PromptRole::Review,
                "schema",
                &["k"],
                "model-family",
                "eval-set",
                body,
            )
            .expect("valid spec");
            registry.register(built).expect("registration");
            let result = registry
                .render(&format!("malformed-{index}"), &[("name", "Ada")])
                .unwrap_err();
            assert!(
                matches!(result, PromptError::MalformedPlaceholder(..)),
                "template `{body}` must be a malformed placeholder, got {result:?}"
            );
        }
    }

    #[test]
    fn validate_output_reports_missing_and_extra_keys() {
        let mut registry = Registry::new();
        let built = spec("out", PromptRole::Evidence, "1.0", &["fact", "source"]);
        registry.register(built).expect("registration");
        assert_eq!(
            registry.validate_output("out", &["fact"]).unwrap_err(),
            PromptError::MissingOutputKey("source".to_string())
        );
        assert_eq!(
            registry
                .validate_output("out", &["fact", "source", "extra"])
                .unwrap_err(),
            PromptError::ExtraOutputKey("extra".to_string())
        );
        registry
            .validate_output("out", &["fact", "source"])
            .expect("the exact declared keys are accepted");
    }

    #[test]
    fn constructor_rejects_duplicate_output_keys() {
        let result = PromptSpec::new(
            "dup",
            Version::parse("1.0").expect("valid version"),
            PromptRole::System,
            "schema",
            &["a", "b", "a"],
            "model-family",
            "eval-set",
            "template",
        );
        assert_eq!(
            result.unwrap_err(),
            PromptError::DuplicateOutputKey("a".to_string())
        );
    }

    #[test]
    fn validate_output_rejects_duplicate_keys() {
        let mut registry = Registry::new();
        registry
            .register(spec(
                "out",
                PromptRole::Evidence,
                "1.0",
                &["fact", "source"],
            ))
            .expect("registration");
        assert_eq!(
            registry
                .validate_output("out", &["fact", "source", "fact"])
                .unwrap_err(),
            PromptError::DuplicateOutputKey("fact".to_string())
        );
    }
}
