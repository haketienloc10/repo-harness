use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseHarnessValueError {
    #[error("unknown intake type '{0}'. Use: new spec, spec slice, change request, new initiative, maintenance request, or harness improvement")]
    InputType(String),
    #[error("unknown lane '{0}'. Use: tiny, normal, or high-risk. Use tiny instead of low.")]
    RiskLane(String),
    #[error("{0} must be an integer")]
    Integer(String),
    #[error("{0} must be 0 or 1. Example: --unit 1 --integration 1 --e2e 0 --platform 0")]
    BoolFlag(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputType {
    NewSpec,
    SpecSlice,
    ChangeRequest,
    NewInitiative,
    Maintenance,
    HarnessImprovement,
}

impl InputType {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::NewSpec => "new_spec",
            Self::SpecSlice => "spec_slice",
            Self::ChangeRequest => "change_request",
            Self::NewInitiative => "new_initiative",
            Self::Maintenance => "maintenance",
            Self::HarnessImprovement => "harness_improvement",
        }
    }
}

impl FromStr for InputType {
    type Err = ParseHarnessValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = normalize_token(value);
        match normalized.as_str() {
            "new_spec" => Ok(Self::NewSpec),
            "spec_slice" => Ok(Self::SpecSlice),
            "change_request" => Ok(Self::ChangeRequest),
            "new_initiative" => Ok(Self::NewInitiative),
            "maintenance" | "maintenance_request" => Ok(Self::Maintenance),
            "harness_improvement" => Ok(Self::HarnessImprovement),
            _ => Err(ParseHarnessValueError::InputType(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RiskLane {
    Tiny,
    Normal,
    HighRisk,
}

impl RiskLane {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Normal => "normal",
            Self::HighRisk => "high_risk",
        }
    }
}

impl FromStr for RiskLane {
    type Err = ParseHarnessValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = normalize_token(value);
        match normalized.as_str() {
            "tiny" => Ok(Self::Tiny),
            "normal" => Ok(Self::Normal),
            "high_risk" => Ok(Self::HighRisk),
            _ => Err(ParseHarnessValueError::RiskLane(value.to_owned())),
        }
    }
}

pub const RISK_LANE_HELP: &str =
    "Accepted lanes: tiny, normal, high-risk. Use tiny instead of low.";

pub const RESPONSIBILITIES: &[&str] = &[
    "Task specification",
    "Context selection",
    "Tool access",
    "Project memory",
    "Task state",
    "Observability",
    "Failure attribution",
    "Verification",
    "Permissions",
    "Entropy auditing",
    "Intervention recording",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolArgSpec {
    pub name: String,
    pub arg_type: String,
    pub required: bool,
    pub help: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolEntry {
    pub provider: String,
    pub name: String,
    pub command: String,
    pub description: String,
    pub args: Vec<ToolArgSpec>,
    pub responsibility: String,
    pub source: String,
    pub since: String,
    /// How the tool is reached and probed: builtin, cli, binary, mcp, skill, http.
    pub kind: String,
    /// Workflow purpose a step looks the tool up by (inbound tools only).
    pub capability: Option<String>,
    /// Declarative thing `tool check` resolves to decide presence.
    pub scan_target: Option<String>,
    /// Last scanned verdict: present, missing, or unknown.
    pub status: String,
    /// When `tool check` last scanned this tool.
    pub checked_at: Option<String>,
}

/// Kinds an inbound tool can register as. `cli`/`binary` are exec-probed on
/// PATH; `mcp`/`skill`/`http` are scanned via their declarative `scan_target`.
pub const TOOL_KINDS: &[&str] = &["cli", "binary", "mcp", "skill", "http"];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolValidationError {
    #[error("--description must be 10-200 characters")]
    DescriptionLength,
    #[error("unknown responsibility '{0}'. Use: {1}")]
    Responsibility(String, String),
    #[error("invalid --args spec '{0}'. Use name:type:required or name:type:required:help")]
    ArgSpec(String),
    #[error("unknown --kind '{0}'. Use: {1}")]
    Kind(String, String),
    #[error(
        "invalid --capability '{0}'. Use kebab-case: lowercase letters, digits, single hyphens"
    )]
    Capability(String),
}

pub fn parse_tool_args(value: Option<String>) -> Result<Vec<ToolArgSpec>, ToolValidationError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }

    value
        .split(',')
        .map(|raw| {
            let parts = raw.splitn(4, ':').map(str::trim).collect::<Vec<_>>();
            if parts.len() < 3
                || parts[0].is_empty()
                || parts[1].is_empty()
                || !matches!(parts[2], "required" | "optional")
            {
                return Err(ToolValidationError::ArgSpec(raw.to_owned()));
            }
            Ok(ToolArgSpec {
                name: parts[0].to_owned(),
                arg_type: parts[1].to_owned(),
                required: parts[2] == "required",
                help: parts
                    .get(3)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string()),
            })
        })
        .collect()
}

pub fn validate_tool_description(description: &str) -> Result<(), ToolValidationError> {
    let length = description.trim().chars().count();
    if !(10..=200).contains(&length) {
        return Err(ToolValidationError::DescriptionLength);
    }
    Ok(())
}

pub fn validate_responsibility(value: &str) -> Result<String, ToolValidationError> {
    RESPONSIBILITIES
        .iter()
        .find(|item| normalize_token(item) == normalize_token(value))
        .map(|item| (*item).to_owned())
        .ok_or_else(|| {
            ToolValidationError::Responsibility(value.to_owned(), RESPONSIBILITIES.join(", "))
        })
}

pub fn validate_tool_kind(value: &str) -> Result<String, ToolValidationError> {
    let normalized = value.trim().to_lowercase();
    TOOL_KINDS
        .iter()
        .find(|kind| **kind == normalized)
        .map(|kind| (*kind).to_owned())
        .ok_or_else(|| ToolValidationError::Kind(value.to_owned(), TOOL_KINDS.join(", ")))
}

/// Capability is intentionally an open, format-validated vocabulary rather than
/// a closed list: the registry is the base for arbitrary future extensions, so
/// new capabilities must not require a code change. Normalizing to kebab-case
/// keeps step lookups (`query tools --capability X`) reliable despite the
/// freedom. A recommended starter vocabulary lives in docs/TOOL_REGISTRY.md.
pub fn normalize_capability(value: &str) -> Result<String, ToolValidationError> {
    let normalized = value.trim().to_lowercase().replace([' ', '_'], "-");
    let well_formed = !normalized.is_empty()
        && normalized
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        && !normalized.starts_with('-')
        && !normalized.ends_with('-')
        && !normalized.contains("--");
    if !well_formed {
        return Err(ToolValidationError::Capability(value.to_owned()));
    }
    Ok(normalized)
}

pub fn compiled_tool_registry() -> Vec<ToolEntry> {
    vec![
        tool(
            "harness-cli",
            "init",
            "init",
            "Create the harness database.",
            &[],
            "Task state",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "migrate",
            "migrate",
            "Apply pending schema migrations.",
            &[],
            "Task state",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "import brownfield",
            "import brownfield",
            "Seed durable records from markdown state.",
            &[],
            "Project memory",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "knowledge scaffold",
            "knowledge scaffold",
            "Create or refresh the repository Knowledge Index.",
            &[],
            "Context selection",
            "0.1.9",
        ),
        tool(
            "harness-cli",
            "knowledge check",
            "knowledge check",
            "Verify the repository Knowledge Index structure and authored content.",
            &[],
            "Context selection",
            "0.1.9",
        ),
        tool(
            "harness-cli",
            "intake",
            "intake",
            "Record a feature intake classification.",
            &[
                ("type", "string", true),
                ("summary", "string", true),
                ("lane", "enum", true),
            ],
            "Task specification",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "story add",
            "story add",
            "Create a durable story record.",
            &[
                ("id", "string", true),
                ("title", "string", true),
                ("lane", "enum", true),
            ],
            "Task state",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "story update",
            "story update",
            "Update story status, proof flags, or verification command.",
            &[("id", "string", true)],
            "Task state",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "story verify",
            "story verify",
            "Run one story verify_command and record pass or fail.",
            &[("id", "string", true)],
            "Verification",
            "0.1.6",
        ),
        tool(
            "harness-cli",
            "story verify-all",
            "story verify-all",
            "Run every configured story verification command.",
            &[],
            "Verification",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "decision add",
            "decision add",
            "Create a durable decision record.",
            &[("id", "string", true), ("title", "string", true)],
            "Project memory",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "decision verify",
            "decision verify",
            "Run one decision verification command.",
            &[("id", "string", true)],
            "Verification",
            "0.1.6",
        ),
        tool(
            "harness-cli",
            "backlog add",
            "backlog add",
            "Record a harness improvement proposal.",
            &[("title", "string", true)],
            "Entropy auditing",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "backlog close",
            "backlog close",
            "Close a backlog item with outcome evidence.",
            &[("id", "integer", true)],
            "Entropy auditing",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "trace",
            "trace",
            "Record an agent execution trace.",
            &[("summary", "string", true)],
            "Observability",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "score-trace",
            "score-trace",
            "Score trace detail against lane requirements.",
            &[("id", "integer", false)],
            "Observability",
            "0.1.4",
        ),
        tool(
            "harness-cli",
            "score-context",
            "score-context",
            "Score trace context reads against context rules.",
            &[("trace-id", "integer", true)],
            "Context selection",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "audit",
            "audit",
            "Run drift checks and compute entropy score.",
            &[],
            "Entropy auditing",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "propose",
            "propose",
            "Generate harness improvement proposals from observed patterns.",
            &[("commit", "flag", false)],
            "Entropy auditing",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "query matrix",
            "query matrix",
            "Show durable story proof matrix.",
            &[],
            "Task state",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "query backlog",
            "query backlog",
            "Show harness improvement backlog.",
            &[],
            "Entropy auditing",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "query decisions",
            "query decisions",
            "Show durable decision records.",
            &[],
            "Project memory",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "query intakes",
            "query intakes",
            "Show recent intake records.",
            &[],
            "Task specification",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "query traces",
            "query traces",
            "Show recent trace records.",
            &[],
            "Observability",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "query friction",
            "query friction",
            "Show traces that recorded harness friction.",
            &[],
            "Failure attribution",
            "0.1.4",
        ),
        tool(
            "harness-cli",
            "query interventions",
            "query interventions",
            "Show human or review intervention records.",
            &[],
            "Intervention recording",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "query stats",
            "query stats",
            "Show durable record counts.",
            &[],
            "Task state",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "query tools",
            "query tools",
            "Show compiled and registered tool manifest entries.",
            &[],
            "Tool access",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "query sql",
            "query sql",
            "Run arbitrary SQL against harness.db.",
            &[("query", "string", true)],
            "Tool access",
            "0.1.0",
        ),
        tool(
            "harness-cli",
            "tool register",
            "tool register",
            "Register an external project tool.",
            &[("name", "string", true), ("command", "string", true)],
            "Tool access",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "tool remove",
            "tool remove",
            "Remove a registered external tool.",
            &[("name", "string", true)],
            "Tool access",
            "0.1.8",
        ),
        tool(
            "harness-cli",
            "intervention add",
            "intervention add",
            "Record a human or review intervention.",
            &[
                ("type", "enum", true),
                ("description", "string", true),
                ("source", "enum", true),
            ],
            "Intervention recording",
            "0.1.8",
        ),
    ]
}

fn tool(
    provider: &str,
    name: &str,
    command: &str,
    description: &str,
    args: &[(&str, &str, bool)],
    responsibility: &str,
    since: &str,
) -> ToolEntry {
    ToolEntry {
        provider: provider.to_owned(),
        name: name.to_owned(),
        command: command.to_owned(),
        description: description.to_owned(),
        args: args
            .iter()
            .map(|(name, arg_type, required)| ToolArgSpec {
                name: (*name).to_owned(),
                arg_type: (*arg_type).to_owned(),
                required: *required,
                help: None,
            })
            .collect(),
        responsibility: responsibility.to_owned(),
        source: "compiled".to_owned(),
        since: since.to_owned(),
        kind: "builtin".to_owned(),
        capability: None,
        scan_target: None,
        status: "present".to_owned(),
        checked_at: None,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IntakeRecord {
    pub id: i64,
    pub created_at: String,
    pub input_type: String,
    pub risk_lane: String,
    pub summary: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StoryMatrixRecord {
    pub id: String,
    pub title: String,
    pub status: String,
    pub unit: i64,
    pub integration: i64,
    pub e2e: i64,
    pub platform: i64,
    pub evidence: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StoryVerifyStatus {
    pub id: String,
    pub verify_command: Option<String>,
    pub last_verified_result: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StoryVerifyAllItem {
    pub id: String,
    pub title: String,
    pub command: Option<String>,
    pub result: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StoryVerifyAllResult {
    pub items: Vec<StoryVerifyAllItem>,
}

impl StoryVerifyAllResult {
    pub fn passed(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.result == "pass")
            .count()
    }

    pub fn failed(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.result == "fail")
            .count()
    }

    pub fn skipped(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.result == "skipped")
            .count()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BacklogRecord {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub risk: Option<String>,
    pub predicted_impact: Option<String>,
    pub actual_outcome: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BacklogFilter {
    All,
    Open,
    Closed,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DecisionRecord {
    pub id: String,
    pub title: String,
    pub status: String,
    pub last_verified_at: Option<String>,
    pub last_verified_result: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TraceRecord {
    pub id: i64,
    pub created_at: String,
    pub outcome: Option<String>,
    pub task_summary: String,
    pub harness_friction: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceQualityTier {
    Incomplete = 0,
    Minimal = 1,
    Standard = 2,
    Detailed = 3,
}

impl TraceQualityTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Incomplete => "incomplete",
            Self::Minimal => "minimal",
            Self::Standard => "standard",
            Self::Detailed => "detailed",
        }
    }

    pub fn score(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TraceScoreSource {
    pub id: i64,
    pub task_summary: String,
    pub intake_id: Option<i64>,
    pub risk_lane: Option<String>,
    pub agent: Option<String>,
    pub actions_taken: Option<String>,
    pub files_read: Option<String>,
    pub files_changed: Option<String>,
    pub decisions_made: Option<String>,
    pub errors: Option<String>,
    pub outcome: Option<String>,
    pub duration_seconds: Option<i64>,
    pub token_estimate: Option<i64>,
    pub harness_friction: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TraceScoreResult {
    pub trace_id: i64,
    pub achieved: TraceQualityTier,
    pub risk_lane: Option<String>,
    pub required: Option<TraceQualityTier>,
    pub meets_requirement: bool,
    pub missing_minimal: Vec<String>,
    pub missing_standard: Vec<String>,
    pub missing_detailed: Vec<String>,
}

pub fn required_trace_tier_for_lane(risk_lane: &str) -> Option<TraceQualityTier> {
    match risk_lane {
        "tiny" => Some(TraceQualityTier::Minimal),
        "normal" => Some(TraceQualityTier::Standard),
        "high_risk" => Some(TraceQualityTier::Detailed),
        _ => None,
    }
}

pub fn score_trace(source: TraceScoreSource) -> TraceScoreResult {
    let missing_minimal = missing_minimal_fields(&source);
    let missing_standard = if missing_minimal.is_empty() {
        missing_standard_fields(&source)
    } else {
        Vec::new()
    };
    let missing_detailed = if missing_minimal.is_empty() && missing_standard.is_empty() {
        missing_detailed_fields(&source)
    } else {
        Vec::new()
    };

    let achieved = if !missing_minimal.is_empty() {
        TraceQualityTier::Incomplete
    } else if !missing_standard.is_empty() {
        TraceQualityTier::Minimal
    } else if !missing_detailed.is_empty() {
        TraceQualityTier::Standard
    } else {
        TraceQualityTier::Detailed
    };
    let required = source
        .risk_lane
        .as_deref()
        .and_then(required_trace_tier_for_lane);
    let meets_requirement = required.is_none_or(|tier| achieved >= tier);

    TraceScoreResult {
        trace_id: source.id,
        achieved,
        risk_lane: source.risk_lane,
        required,
        meets_requirement,
        missing_minimal,
        missing_standard,
        missing_detailed,
    }
}

pub fn score_context(source: ContextScoreSource) -> ContextScoreResult {
    let lane = source
        .risk_lane
        .clone()
        .unwrap_or_else(|| "unknown".to_owned());
    let phase = infer_context_phase(&source);
    let read = jsonish_list(source.files_read.as_deref());
    let changed = jsonish_list(source.files_changed.as_deref());

    let mut must = Vec::new();
    let mut should = Vec::new();
    let mut skipped = Vec::new();

    add_base_context_rules(&lane, &phase, &mut must, &mut should, &mut skipped);
    if changed
        .iter()
        .any(|path| path.starts_with("scripts/schema/"))
    {
        must.push((
            "SQLite durable layer decision",
            "docs/decisions/0004-sqlite-durable-layer.md",
        ));
    }
    if changed
        .iter()
        .any(|path| path.starts_with("crates/harness-cli/") || path.starts_with("scripts/bin/"))
    {
        must.push((
            "Prebuilt CLI decision",
            "docs/decisions/0005-prebuilt-rust-harness-cli.md",
        ));
    }

    let must = must
        .into_iter()
        .map(|(label, target)| ContextRequirementResult {
            label: label.to_owned(),
            target: target.to_owned(),
            met: path_read(&read, target, &changed),
        })
        .collect::<Vec<_>>();
    let should = should
        .into_iter()
        .map(|(label, target)| ContextRequirementResult {
            label: label.to_owned(),
            target: target.to_owned(),
            met: path_read(&read, target, &changed),
        })
        .collect::<Vec<_>>();
    let over_read = read
        .into_iter()
        .filter(|path| skipped.iter().any(|skip| path_matches(path, skip)))
        .collect::<Vec<_>>();

    ContextScoreResult {
        trace_id: source.id,
        lane,
        phase,
        must,
        should,
        over_read,
    }
}

fn infer_context_phase(source: &ContextScoreSource) -> String {
    let changed = source.files_changed.as_deref().unwrap_or("").trim();
    if source.outcome.as_deref() == Some("completed") {
        "trace".to_owned()
    } else if source.story_id.is_some() && !changed.is_empty() && changed != "[]" {
        "implementation".to_owned()
    } else if source.risk_lane.is_some() {
        "planning".to_owned()
    } else {
        "intake".to_owned()
    }
}

fn add_base_context_rules<'a>(
    lane: &str,
    phase: &str,
    must: &mut Vec<(&'a str, &'a str)>,
    should: &mut Vec<(&'a str, &'a str)>,
    skipped: &mut Vec<&'a str>,
) {
    match phase {
        "trace" => {
            must.push(("Trace specification", "docs/TRACE_SPEC.md"));
            must.push(("Changed-file list", "git status --short"));
            if lane == "normal" || lane == "high_risk" {
                must.push(("Durable matrix", "scripts/bin/harness-cli query matrix"));
            } else {
                should.push(("Durable matrix", "scripts/bin/harness-cli query matrix"));
            }
        }
        "implementation" => {
            must.push(("Files being changed", "<changed-files>"));
            if lane == "normal" || lane == "high_risk" {
                must.push(("Relevant story packet", "docs/stories/"));
                should.push(("Architecture rules", "docs/ARCHITECTURE.md"));
            }
            if lane == "high_risk" {
                must.push(("Architecture rules", "docs/ARCHITECTURE.md"));
                must.push((
                    "High-risk story template",
                    "docs/templates/high-risk-story/",
                ));
            }
        }
        "planning" => {
            must.push(("Files to edit", "<changed-files>"));
            if lane == "normal" || lane == "high_risk" {
                must.push(("Story template", "docs/templates/story.md"));
                must.push(("Test matrix", "docs/TEST_MATRIX.md"));
            }
            if lane == "high_risk" {
                must.push((
                    "High-risk story template",
                    "docs/templates/high-risk-story/",
                ));
                must.push(("Harness maturity", "docs/HARNESS_MATURITY.md"));
            }
        }
        _ => {
            must.push(("Agent entrypoint", "AGENTS.md"));
            must.push(("Feature intake", "docs/FEATURE_INTAKE.md"));
            must.push(("Durable matrix", "scripts/bin/harness-cli query matrix"));
            if lane == "tiny" {
                skipped.push("docs/ARCHITECTURE.md");
            } else {
                must.push(("README", "README.md"));
                must.push(("Harness operating model", "docs/HARNESS.md"));
            }
        }
    }
}

fn path_read(read: &[String], target: &str, changed: &[String]) -> bool {
    if target == "<changed-files>" {
        return !changed.is_empty();
    }
    read.iter().any(|path| path_matches(path, target))
}

fn path_matches(path: &str, target: &str) -> bool {
    if target.ends_with('/') {
        path.starts_with(target)
    } else {
        path == target || path.contains(target)
    }
}

pub fn jsonish_list(value: Option<&str>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_owned())
        .filter(|item| !item.is_empty() && item != "null")
        .collect()
}

fn missing_minimal_fields(source: &TraceScoreSource) -> Vec<String> {
    let mut missing = Vec::new();
    if source.task_summary.trim().len() < 10 {
        missing.push("task_summary: missing or shorter than 10 characters".to_owned());
    }
    if blank(&source.outcome) {
        missing.push("outcome: null".to_owned());
    }
    missing
}

fn missing_standard_fields(source: &TraceScoreSource) -> Vec<String> {
    let mut missing = Vec::new();
    if blank(&source.agent) {
        missing.push("agent: empty".to_owned());
    }
    if short_json_list(&source.actions_taken) {
        missing.push("actions_taken: empty".to_owned());
    }
    if short_json_list(&source.files_read) {
        missing.push("files_read: empty".to_owned());
    }
    if source.files_changed.is_none() {
        missing.push("files_changed: null".to_owned());
    }
    if source.errors.is_none() && source.harness_friction.is_none() {
        missing.push("errors or harness_friction: both null".to_owned());
    }
    missing
}

fn missing_detailed_fields(source: &TraceScoreSource) -> Vec<String> {
    let mut missing = Vec::new();
    if short_json_list(&source.decisions_made) {
        missing.push("decisions_made: empty".to_owned());
    }
    if source.errors.is_none() {
        missing.push("errors: null".to_owned());
    }
    if source.harness_friction.is_none() {
        missing.push("harness_friction: null".to_owned());
    }
    if source.duration_seconds.is_none() && !notes_explain_missing(&source.notes, "duration") {
        missing.push("duration_seconds: null (no explanation in notes)".to_owned());
    }
    if source.token_estimate.is_none() && !notes_explain_missing(&source.notes, "token") {
        missing.push("token_estimate: null (no explanation in notes)".to_owned());
    }
    missing
}

fn blank(value: &Option<String>) -> bool {
    value.as_deref().map(str::trim).unwrap_or("").is_empty()
}

fn short_json_list(value: &Option<String>) -> bool {
    value.as_deref().map(str::trim).unwrap_or("").len() <= 2
}

fn notes_explain_missing(notes: &Option<String>, field: &str) -> bool {
    let Some(notes) = notes.as_deref() else {
        return false;
    };
    let lower = notes.to_ascii_lowercase();
    lower.contains(field)
        && (lower.contains("unavailable")
            || lower.contains("not available")
            || lower.contains("unknown"))
}

#[derive(Debug, PartialEq, Eq)]
pub struct FrictionRecord {
    pub id: i64,
    pub created_at: String,
    pub risk_lane: Option<String>,
    pub input_type: Option<String>,
    pub task_summary: String,
    pub harness_friction: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InterventionRecord {
    pub id: i64,
    pub created_at: String,
    pub trace_id: Option<i64>,
    pub story_id: Option<String>,
    pub intervention_type: String,
    pub description: String,
    pub source: String,
    pub impact: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContextScoreSource {
    pub id: i64,
    pub risk_lane: Option<String>,
    pub story_id: Option<String>,
    pub files_read: Option<String>,
    pub files_changed: Option<String>,
    pub outcome: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContextRequirementResult {
    pub label: String,
    pub target: String,
    pub met: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContextScoreResult {
    pub trace_id: i64,
    pub lane: String,
    pub phase: String,
    pub must: Vec<ContextRequirementResult>,
    pub should: Vec<ContextRequirementResult>,
    pub over_read: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AuditFinding {
    pub id: String,
    pub title: String,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct AuditResult {
    pub orphaned_stories: Vec<AuditFinding>,
    pub unverified_stories: Vec<AuditFinding>,
    pub unverified_decisions: Vec<AuditFinding>,
    pub backlog_without_outcomes: Vec<AuditFinding>,
    pub stale_stories: Vec<AuditFinding>,
    pub broken_tools: Vec<AuditFinding>,
}

impl AuditResult {
    pub fn entropy_score(&self) -> i64 {
        let raw = (self.orphaned_stories.len() as i64 * 10)
            + (self.unverified_stories.len() as i64 * 5)
            + (self.unverified_decisions.len() as i64 * 5)
            + (self.backlog_without_outcomes.len() as i64 * 2)
            + (self.stale_stories.len() as i64 * 3)
            + (self.broken_tools.len() as i64 * 8);
        raw.min(100)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ImprovementProposal {
    pub title: String,
    pub component: String,
    pub evidence: String,
    pub predicted_impact: String,
    pub risk: String,
    pub suggested_action: String,
    pub validation_plan: String,
    pub confidence: String,
    pub committed_backlog_id: Option<i64>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HarnessStats {
    pub intakes: i64,
    pub stories: i64,
    pub decisions: i64,
    pub backlog_items: i64,
    pub traces: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvList(pub Option<String>);

impl CsvList {
    pub fn from_optional(value: Option<String>) -> Self {
        Self(value.filter(|item| !item.is_empty()))
    }

    pub fn as_json_text(&self) -> Option<String> {
        self.0.as_ref().map(|value| {
            let escaped_items = value
                .split(',')
                .map(|item| format!("\"{}\"", escape_json_string(item.trim())))
                .collect::<Vec<_>>()
                .join(",");
            format!("[{escaped_items}]")
        })
    }

    pub fn as_json_text_or_null_literal(&self) -> String {
        self.as_json_text().unwrap_or_else(|| "null".to_owned())
    }
}

impl fmt::Display for CsvList {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_json_text_or_null_literal())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoolFlag(pub i64);

impl BoolFlag {
    pub fn parse(label: &str, value: &str) -> Result<Self, ParseHarnessValueError> {
        match value {
            "0" => Ok(Self(0)),
            "1" => Ok(Self(1)),
            _ => Err(ParseHarnessValueError::BoolFlag(label.to_owned())),
        }
    }
}

pub fn parse_optional_integer(
    label: &str,
    value: Option<String>,
) -> Result<Option<i64>, ParseHarnessValueError> {
    value
        .map(|inner| {
            inner
                .parse::<i64>()
                .map_err(|_| ParseHarnessValueError::Integer(label.to_owned()))
        })
        .transpose()
}

fn escape_json_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub fn normalize_token(value: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;

    for character in value.trim().chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            normalized.push(character);
            last_was_separator = false;
        } else if !last_was_separator && !normalized.is_empty() {
            normalized.push('_');
            last_was_separator = true;
        }
    }

    while normalized.ends_with('_') {
        normalized.pop();
    }

    normalized
}

pub fn yes_no(value: i64) -> String {
    if value == 1 {
        "yes".to_owned()
    } else {
        "no".to_owned()
    }
}

pub fn proof_display(value: i64, numeric: bool) -> String {
    if numeric {
        value.to_string()
    } else {
        yes_no(value)
    }
}

/// Pure logic for the repository Knowledge Index ("Accessed knowledge" map).
///
/// Filesystem reads and writes live in `infrastructure`; this module only
/// transforms already-gathered inputs into the rendered markdown and back.
pub mod knowledge {
    use std::collections::{BTreeMap, BTreeSet};

    pub const INDEX_PATH: &str = "docs/KNOWLEDGE_INDEX.md";

    pub const PURPOSE_BEGIN: &str = "<!-- KNOWLEDGE:PURPOSE:BEGIN -->";
    pub const PURPOSE_END: &str = "<!-- KNOWLEDGE:PURPOSE:END -->";
    pub const CONCEPTS_BEGIN: &str = "<!-- KNOWLEDGE:CONCEPTS:BEGIN -->";
    pub const CONCEPTS_END: &str = "<!-- KNOWLEDGE:CONCEPTS:END -->";

    const STRUCTURE_SEPARATOR: &str = "—";
    const PURPOSE_PLACEHOLDER: &str =
        "TODO: Describe what this repository is for in 1-3 sentences (Purpose).";
    const CONCEPTS_PLACEHOLDER: &str =
        "TODO: List the core concepts and terms an agent must know. See docs/GLOSSARY.md.";
    const DESC_PLACEHOLDER: &str = "TODO: describe.";

    const HEADING_PURPOSE: &str = "## Purpose";
    const HEADING_TECHNOLOGIES: &str = "## Key Technologies";
    const HEADING_HOWTORUN: &str = "## How to Run";
    const HEADING_STRUCTURE: &str = "## Top-Level Structure";
    const HEADING_SUBDIRS: &str = "## Key Subdirectories";
    const HEADING_CONCEPTS: &str = "## Key Concepts";

    const HOWTORUN_NONE: &str = "No standard build/test commands detected.";
    const SUBDIRS_NONE: &str = "None.";

    /// Signal tokens emitted by infrastructure for technology detection.
    /// Top-level entry names are passed verbatim; computed tokens use these.
    pub const SIGNAL_CARGO_WORKSPACE: &str = "cargo-workspace";
    pub const SIGNAL_RUST_SQLITE: &str = "rust-sqlite";

    /// Framework signals derived from manifest contents (e.g. `dep:react`,
    /// emitted by infrastructure) mapped to display labels. Order defines
    /// render order.
    const FRAMEWORK_SIGNALS: &[(&str, &str)] = &[
        ("dep:react", "React"),
        ("dep:next", "Next.js"),
        ("dep:vue", "Vue"),
        ("dep:angular", "Angular"),
        ("dep:svelte", "Svelte"),
        ("dep:express", "Express"),
        ("dep:nestjs", "NestJS"),
        ("dep:django", "Django"),
        ("dep:flask", "Flask"),
        ("dep:fastapi", "FastAPI"),
        ("dep:rails", "Ruby on Rails"),
    ];

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TopLevelEntry {
        pub name: String,
        pub is_dir: bool,
    }

    /// A build/test/run command derived from a manifest, with a short label.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RunCommand {
        pub command: String,
        pub label: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct KnowledgeInputs {
        pub repo_name: String,
        pub technologies: Vec<String>,
        pub entries: Vec<TopLevelEntry>,
        /// Immediate subdirectories of each top-level directory (one level
        /// deeper than `entries`), addressed by relative path in `name`.
        pub subdirectories: Vec<TopLevelEntry>,
        /// Deterministic build/test/run commands derived from manifests.
        pub commands: Vec<RunCommand>,
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    pub struct PreservedIndex {
        pub purpose: Option<String>,
        pub concepts: Option<String>,
        pub structure_descriptions: BTreeMap<String, String>,
        pub subdirectory_descriptions: BTreeMap<String, String>,
    }

    /// Map a set of signal tokens to a stable, de-duplicated technology list.
    pub fn detect_technologies(signals: &BTreeSet<String>) -> Vec<String> {
        let has = |token: &str| signals.contains(token);
        let mut technologies: Vec<String> = Vec::new();
        let push = |technologies: &mut Vec<String>, label: &str| {
            if !technologies.iter().any(|item| item == label) {
                technologies.push(label.to_owned());
            }
        };

        if has("Cargo.toml") || has("ext:rs") {
            push(&mut technologies, "Rust");
        }
        if has(SIGNAL_CARGO_WORKSPACE) {
            push(&mut technologies, "Cargo Workspace");
        }
        if has("ext:sql") {
            if has(SIGNAL_RUST_SQLITE) {
                push(&mut technologies, "SQLite");
            } else {
                push(&mut technologies, "SQL");
            }
        }
        if has("package.json") {
            push(&mut technologies, "Node.js");
        }
        if has("tsconfig.json") || has("ext:ts") {
            push(&mut technologies, "TypeScript");
        }
        if has("pyproject.toml") || has("requirements.txt") || has("ext:py") {
            push(&mut technologies, "Python");
        }
        if has("go.mod") || has("ext:go") {
            push(&mut technologies, "Go");
        }
        if has("pom.xml") || has("build.gradle") || has("build.gradle.kts") || has("ext:java") {
            push(&mut technologies, "Java");
        }
        if has("ext:kt") || has("build.gradle.kts") {
            push(&mut technologies, "Kotlin");
        }
        if has("Package.swift") || has("ext:swift") {
            push(&mut technologies, "Swift");
        }
        if has("Gemfile") || has("ext:rb") {
            push(&mut technologies, "Ruby");
        }
        if has("composer.json") || has("ext:php") {
            push(&mut technologies, "PHP");
        }
        if has("ext:cpp") || has("ext:cc") || has("ext:cxx") || has("ext:hpp") {
            push(&mut technologies, "C++");
        }
        if has("ext:c") || has("ext:h") {
            push(&mut technologies, "C");
        }
        if has("ext:cs") || has("ext:csproj") || has("ext:sln") {
            push(&mut technologies, ".NET");
        }
        if has("ext:tf") {
            push(&mut technologies, "Terraform");
        }
        // Node package manager (only meaningful when a package.json is present).
        if has("package.json") {
            if has("pnpm-lock.yaml") {
                push(&mut technologies, "pnpm");
            } else if has("yarn.lock") {
                push(&mut technologies, "Yarn");
            } else if has("package-lock.json") {
                push(&mut technologies, "npm");
            }
        }
        // Frameworks detected from manifest contents (dep:* signals).
        for (signal, label) in FRAMEWORK_SIGNALS {
            if has(signal) {
                push(&mut technologies, label);
            }
        }
        if has(".prettierrc") || has(".prettierignore") {
            push(&mut technologies, "Prettier");
        }
        if has(".editorconfig") {
            push(&mut technologies, "EditorConfig");
        }
        if has("Dockerfile") || has("docker-compose.yml") {
            push(&mut technologies, "Docker");
        }
        if has("ext:sh") {
            push(&mut technologies, "Bash");
        }
        if has("ext:md") {
            push(&mut technologies, "Markdown");
        }

        technologies
    }

    /// Extract authored blocks and per-entry structure descriptions from an
    /// existing index so a regeneration can preserve them.
    pub fn parse_preserved(content: &str) -> PreservedIndex {
        PreservedIndex {
            purpose: extract_between(content, PURPOSE_BEGIN, PURPOSE_END),
            concepts: extract_between(content, CONCEPTS_BEGIN, CONCEPTS_END),
            structure_descriptions: parse_structure_descriptions(content),
            subdirectory_descriptions: parse_entry_descriptions(content, HEADING_SUBDIRS),
        }
    }

    /// Render the full index, regenerating deterministic sections and
    /// re-inserting any preserved authored content.
    pub fn render_index(inputs: &KnowledgeInputs, preserved: &PreservedIndex) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Knowledge Index — {}\n\n", inputs.repo_name));
        out.push_str(
            "> \"Accessed knowledge\": the onboarding map agents read before changing code.\n",
        );
        out.push_str(
            "> Generated by `harness-cli knowledge`. Key Technologies, How to Run, Top-Level\n",
        );
        out.push_str(
            "> Structure and Key Subdirectories are regenerated each run; Purpose, Key Concepts\n",
        );
        out.push_str(
            "> and per-entry descriptions are authored and preserved between the markers.\n\n",
        );

        out.push_str(HEADING_PURPOSE);
        out.push_str("\n\n");
        out.push_str(PURPOSE_BEGIN);
        out.push('\n');
        out.push_str(preserved_or(
            preserved.purpose.as_deref(),
            PURPOSE_PLACEHOLDER,
        ));
        out.push('\n');
        out.push_str(PURPOSE_END);
        out.push_str("\n\n");

        out.push_str(HEADING_TECHNOLOGIES);
        out.push_str("\n\n");
        if inputs.technologies.is_empty() {
            out.push_str("- TODO: no technologies detected.\n");
        } else {
            for technology in &inputs.technologies {
                out.push_str(&format!("- {technology}\n"));
            }
        }
        out.push('\n');

        out.push_str(HEADING_HOWTORUN);
        out.push_str("\n\n");
        if inputs.commands.is_empty() {
            out.push_str(&format!("- {HOWTORUN_NONE}\n"));
        } else {
            for command in &inputs.commands {
                out.push_str(&format!(
                    "- `{}` {STRUCTURE_SEPARATOR} {}\n",
                    command.command, command.label
                ));
            }
        }
        out.push('\n');

        out.push_str(HEADING_STRUCTURE);
        out.push_str("\n\n");
        if inputs.entries.is_empty() {
            out.push_str("- TODO: no entries found.\n");
        } else {
            render_entry_list(&mut out, &inputs.entries, &preserved.structure_descriptions);
        }
        out.push('\n');

        out.push_str(HEADING_SUBDIRS);
        out.push_str("\n\n");
        if inputs.subdirectories.is_empty() {
            out.push_str(&format!("- {SUBDIRS_NONE}\n"));
        } else {
            render_entry_list(
                &mut out,
                &inputs.subdirectories,
                &preserved.subdirectory_descriptions,
            );
        }
        out.push('\n');

        out.push_str(HEADING_CONCEPTS);
        out.push_str("\n\n");
        out.push_str(CONCEPTS_BEGIN);
        out.push('\n');
        out.push_str(preserved_or(
            preserved.concepts.as_deref(),
            CONCEPTS_PLACEHOLDER,
        ));
        out.push('\n');
        out.push_str(CONCEPTS_END);
        out.push('\n');

        out
    }

    /// Render a `- `path/` — description` list, falling back to the TODO
    /// placeholder for entries without a preserved description.
    fn render_entry_list(
        out: &mut String,
        entries: &[TopLevelEntry],
        descriptions: &BTreeMap<String, String>,
    ) {
        for entry in entries {
            let display = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            let description = descriptions
                .get(&entry.name)
                .map(String::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(DESC_PLACEHOLDER);
            out.push_str(&format!(
                "- `{display}` {STRUCTURE_SEPARATOR} {description}\n"
            ));
        }
    }

    /// Mechanical VERIFY gate: returns a list of problems (empty == healthy).
    pub fn check_index(existing: Option<&str>, inputs: &KnowledgeInputs) -> Vec<String> {
        let mut problems = Vec::new();
        let Some(content) = existing else {
            problems.push(format!(
                "{INDEX_PATH} is missing. Run: harness-cli knowledge scaffold"
            ));
            return problems;
        };

        for heading in [
            HEADING_PURPOSE,
            HEADING_TECHNOLOGIES,
            HEADING_HOWTORUN,
            HEADING_STRUCTURE,
            HEADING_SUBDIRS,
            HEADING_CONCEPTS,
        ] {
            if !has_heading(content, heading) {
                problems.push(format!("missing section: {heading}"));
            }
        }

        let preserved = parse_preserved(content);
        check_authored(&mut problems, "Purpose", preserved.purpose.as_deref());
        check_authored(&mut problems, "Key Concepts", preserved.concepts.as_deref());

        // The Technologies section is regenerated, but an empty list still
        // renders a `TODO` placeholder; flag it so `check` matches the
        // documented contract (no remaining TODO placeholders).
        if inputs.technologies.is_empty() {
            problems.push(
                "Key Technologies has no detected entries (TODO placeholder). \
                 Improve detection heuristics or add a recognizable signal file."
                    .to_owned(),
            );
        }

        check_entry_section(
            &mut problems,
            "Top-Level Structure",
            &preserved.structure_descriptions,
            &inputs.entries,
        );
        check_entry_section(
            &mut problems,
            "Key Subdirectories",
            &preserved.subdirectory_descriptions,
            &inputs.subdirectories,
        );

        problems
    }

    /// Compare a parsed description map against the entries currently on disk
    /// and flag drift (missing/extra) plus any remaining TODO descriptions.
    fn check_entry_section(
        problems: &mut Vec<String>,
        label: &str,
        descriptions: &BTreeMap<String, String>,
        entries: &[TopLevelEntry],
    ) {
        let parsed_names: BTreeSet<String> = descriptions.keys().cloned().collect();
        let current_names: BTreeSet<String> =
            entries.iter().map(|entry| entry.name.clone()).collect();
        for missing in current_names.difference(&parsed_names) {
            problems.push(format!(
                "{label} is stale: missing entry `{missing}`. Run: harness-cli knowledge scaffold"
            ));
        }
        for extra in parsed_names.difference(&current_names) {
            problems.push(format!(
                "{label} lists `{extra}` which no longer exists. Run: harness-cli knowledge scaffold"
            ));
        }
        for (name, description) in descriptions {
            if description.contains("TODO") {
                problems.push(format!(
                    "{label} entry `{name}` still has a TODO description."
                ));
            }
        }
    }

    fn check_authored(problems: &mut Vec<String>, label: &str, value: Option<&str>) {
        match value {
            None => problems.push(format!("{label} markers are missing.")),
            Some(text) if text.trim().is_empty() => problems.push(format!("{label} is empty.")),
            Some(text) if text.contains("TODO") => {
                problems.push(format!("{label} still has a TODO placeholder."))
            }
            Some(_) => {}
        }
    }

    fn preserved_or<'a>(value: Option<&'a str>, fallback: &'a str) -> &'a str {
        value
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or(fallback)
    }

    fn has_heading(content: &str, heading: &str) -> bool {
        content.lines().any(|line| line.trim() == heading)
    }

    fn extract_between(content: &str, begin: &str, end: &str) -> Option<String> {
        let start = content.find(begin)? + begin.len();
        let rest = &content[start..];
        let stop = rest.find(end)?;
        Some(rest[..stop].trim().to_owned())
    }

    fn parse_structure_descriptions(content: &str) -> BTreeMap<String, String> {
        parse_entry_descriptions(content, HEADING_STRUCTURE)
    }

    /// Parse the `- `name` — description` list under `heading` into a map of
    /// name (trailing slash trimmed) to description, joining wrapped lines.
    fn parse_entry_descriptions(content: &str, heading: &str) -> BTreeMap<String, String> {
        let mut descriptions = BTreeMap::new();
        let mut in_section = false;
        let mut current: Option<(String, String)> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == heading {
                in_section = true;
                continue;
            }
            if !in_section {
                continue;
            }
            if trimmed.starts_with("## ") {
                break;
            }

            if let Some((name, first)) = parse_structure_line(trimmed) {
                flush(&mut descriptions, current.take());
                current = Some((name, first));
            } else if trimmed.is_empty() {
                flush(&mut descriptions, current.take());
            } else if let Some((_, description)) = current.as_mut() {
                // Continuation of a description wrapped by the formatter.
                if !description.is_empty() {
                    description.push(' ');
                }
                description.push_str(trimmed);
            }
        }
        flush(&mut descriptions, current.take());
        descriptions
    }

    fn flush(descriptions: &mut BTreeMap<String, String>, entry: Option<(String, String)>) {
        if let Some((name, description)) = entry {
            descriptions.insert(name, description.trim().to_owned());
        }
    }

    fn parse_structure_line(line: &str) -> Option<(String, String)> {
        let rest = line.strip_prefix("- `")?;
        let (name_part, after) = rest.split_once('`')?;
        let after = after.trim_start();
        let description = after
            .strip_prefix(STRUCTURE_SEPARATOR)
            .unwrap_or(after)
            .trim();
        let name = name_part.trim_end_matches('/').to_owned();
        if name.is_empty() {
            return None;
        }
        Some((name, description.to_owned()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn signals(tokens: &[&str]) -> BTreeSet<String> {
            tokens.iter().map(|token| (*token).to_owned()).collect()
        }

        fn sample_inputs() -> KnowledgeInputs {
            KnowledgeInputs {
                repo_name: "demo".to_owned(),
                technologies: vec!["Rust".to_owned()],
                entries: vec![
                    TopLevelEntry {
                        name: "src".to_owned(),
                        is_dir: true,
                    },
                    TopLevelEntry {
                        name: "Cargo.toml".to_owned(),
                        is_dir: false,
                    },
                ],
                subdirectories: Vec::new(),
                commands: Vec::new(),
            }
        }

        #[test]
        fn detects_rust_sqlite_workspace_stack() {
            let detected = detect_technologies(&signals(&[
                "Cargo.toml",
                "ext:rs",
                "ext:sql",
                SIGNAL_CARGO_WORKSPACE,
                SIGNAL_RUST_SQLITE,
                ".prettierrc",
            ]));
            assert_eq!(
                detected,
                vec![
                    "Rust".to_owned(),
                    "Cargo Workspace".to_owned(),
                    "SQLite".to_owned(),
                    "Prettier".to_owned(),
                ]
            );
        }

        #[test]
        fn sql_without_rusqlite_is_generic_sql() {
            let detected = detect_technologies(&signals(&["ext:sql"]));
            assert_eq!(detected, vec!["SQL".to_owned()]);
        }

        #[test]
        fn render_then_parse_round_trips_authored_content() {
            let mut preserved = PreservedIndex {
                purpose: Some("A demo repo.".to_owned()),
                concepts: Some("- **Term** — meaning.".to_owned()),
                ..Default::default()
            };
            preserved
                .structure_descriptions
                .insert("src".to_owned(), "Source code.".to_owned());

            let rendered = render_index(&sample_inputs(), &preserved);
            let reparsed = parse_preserved(&rendered);

            assert_eq!(reparsed.purpose.as_deref(), Some("A demo repo."));
            assert_eq!(reparsed.concepts.as_deref(), Some("- **Term** — meaning."));
            assert_eq!(
                reparsed
                    .structure_descriptions
                    .get("src")
                    .map(String::as_str),
                Some("Source code.")
            );
            // Entry without an authored description falls back to the placeholder.
            assert!(reparsed.structure_descriptions["Cargo.toml"].contains("TODO"));
        }

        #[test]
        fn parse_joins_wrapped_description_lines() {
            let content = "## Top-Level Structure\n\n- `src/` — A long description that the\n  formatter wrapped onto two lines.\n\n## Key Concepts\n";
            let parsed = parse_structure_descriptions(content);
            assert_eq!(
                parsed.get("src").map(String::as_str),
                Some("A long description that the formatter wrapped onto two lines.")
            );
        }

        #[test]
        fn check_reports_missing_then_passes_when_authored() {
            let inputs = sample_inputs();
            assert_eq!(
                check_index(None, &inputs),
                vec![format!(
                    "{INDEX_PATH} is missing. Run: harness-cli knowledge scaffold"
                )]
            );

            // Freshly scaffolded (no preserved content) -> TODO placeholders fail.
            let scaffolded = render_index(&inputs, &PreservedIndex::default());
            assert!(!check_index(Some(&scaffolded), &inputs).is_empty());

            let mut preserved = PreservedIndex {
                purpose: Some("A demo repo.".to_owned()),
                concepts: Some("Core terms.".to_owned()),
                ..Default::default()
            };
            preserved
                .structure_descriptions
                .insert("src".to_owned(), "Source.".to_owned());
            preserved
                .structure_descriptions
                .insert("Cargo.toml".to_owned(), "Manifest.".to_owned());
            let authored = render_index(&inputs, &preserved);
            assert!(check_index(Some(&authored), &inputs).is_empty());
        }

        #[test]
        fn check_detects_structure_drift() {
            let mut preserved = PreservedIndex {
                purpose: Some("A demo repo.".to_owned()),
                concepts: Some("Core terms.".to_owned()),
                ..Default::default()
            };
            preserved
                .structure_descriptions
                .insert("src".to_owned(), "Source.".to_owned());
            preserved
                .structure_descriptions
                .insert("Cargo.toml".to_owned(), "Manifest.".to_owned());
            let authored = render_index(&sample_inputs(), &preserved);

            // A new top-level entry appeared on disk but is absent from the index.
            let mut drifted = sample_inputs();
            drifted.entries.push(TopLevelEntry {
                name: "docs".to_owned(),
                is_dir: true,
            });
            let problems = check_index(Some(&authored), &drifted);
            assert!(problems.iter().any(|problem| problem.contains("`docs`")));
        }

        #[test]
        fn check_flags_empty_technologies_todo() {
            let mut preserved = PreservedIndex {
                purpose: Some("A demo repo.".to_owned()),
                concepts: Some("Core terms.".to_owned()),
                ..Default::default()
            };
            preserved
                .structure_descriptions
                .insert("src".to_owned(), "Source.".to_owned());
            preserved
                .structure_descriptions
                .insert("Cargo.toml".to_owned(), "Manifest.".to_owned());

            // No technologies detected -> render emits a TODO placeholder, so
            // check must report it even though every authored block is filled.
            let mut inputs = sample_inputs();
            inputs.technologies.clear();
            let authored = render_index(&inputs, &preserved);
            let problems = check_index(Some(&authored), &inputs);
            assert!(problems
                .iter()
                .any(|problem| problem.contains("Key Technologies")));
        }

        #[test]
        fn detects_extended_languages_pm_and_frameworks() {
            let detected = detect_technologies(&signals(&[
                "package.json",
                "ext:ts",
                "yarn.lock",
                "dep:react",
                "dep:next",
                "go.mod",
                "pom.xml",
                "Gemfile",
                "dep:rails",
                "ext:tf",
            ]));
            for expected in [
                "Node.js",
                "TypeScript",
                "Go",
                "Java",
                "Ruby",
                "Terraform",
                "Yarn",
                "React",
                "Next.js",
                "Ruby on Rails",
            ] {
                assert!(
                    detected.iter().any(|item| item == expected),
                    "expected {expected} in {detected:?}"
                );
            }
            // package-lock absent + yarn.lock present -> Yarn, not npm.
            assert!(!detected.iter().any(|item| item == "npm"));
        }

        #[test]
        fn how_to_run_renders_commands_without_todo() {
            let mut inputs = sample_inputs();
            inputs.commands = vec![
                RunCommand {
                    command: "cargo build".to_owned(),
                    label: "build".to_owned(),
                },
                RunCommand {
                    command: "cargo test".to_owned(),
                    label: "test".to_owned(),
                },
            ];
            let rendered = render_index(&inputs, &PreservedIndex::default());
            assert!(rendered.contains("## How to Run"));
            assert!(rendered.contains("- `cargo build` — build"));
            // An empty command list renders a neutral, non-TODO line.
            let empty = render_index(&sample_inputs(), &PreservedIndex::default());
            let how_to_run = empty.split("## How to Run").nth(1).unwrap();
            let how_to_run = how_to_run.split("## ").next().unwrap();
            assert!(!how_to_run.contains("TODO"));
        }

        #[test]
        fn subdirectories_round_trip_and_drift_is_detected() {
            let mut inputs = sample_inputs();
            inputs.subdirectories = vec![TopLevelEntry {
                name: "src/app".to_owned(),
                is_dir: true,
            }];

            let mut preserved = PreservedIndex {
                purpose: Some("A demo repo.".to_owned()),
                concepts: Some("Core terms.".to_owned()),
                ..Default::default()
            };
            preserved
                .structure_descriptions
                .insert("src".to_owned(), "Source.".to_owned());
            preserved
                .structure_descriptions
                .insert("Cargo.toml".to_owned(), "Manifest.".to_owned());
            preserved
                .subdirectory_descriptions
                .insert("src/app".to_owned(), "App package.".to_owned());

            let authored = render_index(&inputs, &preserved);
            assert!(authored.contains("- `src/app/` — App package."));
            // Preserved subdir description survives a parse round-trip.
            let reparsed = parse_preserved(&authored);
            assert_eq!(
                reparsed
                    .subdirectory_descriptions
                    .get("src/app")
                    .map(String::as_str),
                Some("App package.")
            );
            assert!(check_index(Some(&authored), &inputs).is_empty());

            // A new subdirectory on disk but absent from the index is drift.
            let mut drifted = inputs.clone();
            drifted.subdirectories.push(TopLevelEntry {
                name: "src/core".to_owned(),
                is_dir: true,
            });
            let problems = check_index(Some(&authored), &drifted);
            assert!(problems
                .iter()
                .any(|problem| problem.contains("Key Subdirectories")
                    && problem.contains("`src/core`")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_input_type_aliases() {
        assert_eq!("new_spec".parse::<InputType>().unwrap(), InputType::NewSpec);
        assert_eq!(
            "maintenance request".parse::<InputType>().unwrap(),
            InputType::Maintenance
        );
        assert_eq!(
            "Harness improvement".parse::<InputType>().unwrap(),
            InputType::HarnessImprovement
        );
    }

    #[test]
    fn parses_high_risk_lane_alias() {
        assert_eq!("high-risk".parse::<RiskLane>().unwrap(), RiskLane::HighRisk);
    }

    #[test]
    fn renders_csv_as_json_text() {
        assert_eq!(
            CsvList::from_optional(Some("auth, data model".to_owned()))
                .as_json_text_or_null_literal(),
            "[\"auth\",\"data model\"]"
        );
        assert_eq!(
            CsvList::from_optional(None).as_json_text_or_null_literal(),
            "null"
        );
    }

    #[test]
    fn parses_bool_flags() {
        assert_eq!(BoolFlag::parse("--unit", "1").unwrap(), BoolFlag(1));
        assert!(BoolFlag::parse("--unit", "yes").is_err());
    }

    fn trace_source() -> TraceScoreSource {
        TraceScoreSource {
            id: 7,
            task_summary: "Completed a useful task".to_owned(),
            intake_id: None,
            risk_lane: None,
            agent: None,
            actions_taken: None,
            files_read: None,
            files_changed: None,
            decisions_made: None,
            errors: None,
            outcome: Some("completed".to_owned()),
            duration_seconds: None,
            token_estimate: None,
            harness_friction: None,
            notes: None,
        }
    }

    #[test]
    fn scores_minimal_standard_and_detailed_traces() {
        let minimal = score_trace(trace_source());
        assert_eq!(minimal.achieved, TraceQualityTier::Minimal);

        let mut standard_source = trace_source();
        standard_source.agent = Some("codex".to_owned());
        standard_source.actions_taken = Some("[\"read\",\"patched\"]".to_owned());
        standard_source.files_read = Some("[\"PHASE3.md\"]".to_owned());
        standard_source.files_changed = Some("[\"docs/TRACE_SPEC.md\"]".to_owned());
        standard_source.harness_friction = Some("none".to_owned());
        let standard = score_trace(standard_source);
        assert_eq!(standard.achieved, TraceQualityTier::Standard);

        let mut detailed_source = trace_source();
        detailed_source.agent = Some("codex".to_owned());
        detailed_source.actions_taken = Some("[\"read\",\"patched\"]".to_owned());
        detailed_source.files_read = Some("[\"PHASE3.md\"]".to_owned());
        detailed_source.files_changed = Some("[\"docs/TRACE_SPEC.md\"]".to_owned());
        detailed_source.decisions_made = Some("[\"kept schema unchanged\"]".to_owned());
        detailed_source.errors = Some("[\"none\"]".to_owned());
        detailed_source.harness_friction = Some("none".to_owned());
        detailed_source.duration_seconds = Some(120);
        detailed_source.token_estimate = Some(2000);
        let detailed = score_trace(detailed_source);
        assert_eq!(detailed.achieved, TraceQualityTier::Detailed);
    }

    #[test]
    fn compares_trace_score_to_lane_requirement() {
        let mut source = trace_source();
        source.risk_lane = Some("high_risk".to_owned());
        source.agent = Some("codex".to_owned());
        source.actions_taken = Some("[\"read\",\"patched\"]".to_owned());
        source.files_read = Some("[\"PHASE3.md\"]".to_owned());
        source.files_changed = Some("[\"docs/TRACE_SPEC.md\"]".to_owned());
        source.harness_friction = Some("none".to_owned());

        let result = score_trace(source);

        assert_eq!(result.achieved, TraceQualityTier::Standard);
        assert_eq!(result.required, Some(TraceQualityTier::Detailed));
        assert!(!result.meets_requirement);
        assert!(result
            .missing_detailed
            .iter()
            .any(|field| field.starts_with("decisions_made")));
    }

    #[test]
    fn context_score_applies_lane_and_retrieval_triggers() {
        let result = score_context(ContextScoreSource {
            id: 42,
            risk_lane: Some("normal".to_owned()),
            story_id: Some("US-019".to_owned()),
            files_read: Some(
                "[\"docs/stories/epics/E03-phase-5-evolution-infrastructure/US-019-tool-registry.md\",\"docs/decisions/0005-prebuilt-rust-harness-cli.md\"]".to_owned(),
            ),
            files_changed: Some("[\"crates/harness-cli/src/interface.rs\"]".to_owned()),
            outcome: None,
        });

        assert_eq!(result.phase, "implementation");
        assert!(result
            .must
            .iter()
            .any(|item| item.target == "docs/stories/" && item.met));
        assert!(result.must.iter().any(|item| item.target
            == "docs/decisions/0005-prebuilt-rust-harness-cli.md"
            && item.met));
    }
}
