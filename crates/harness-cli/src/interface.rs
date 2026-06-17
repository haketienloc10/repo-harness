use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, Parser, Subcommand};
use thiserror::Error;

use crate::application::{
    BacklogAddInput, BacklogCloseInput, BrownfieldImportResult, DecisionAddInput, EvidenceAddInput,
    EvidenceFilter, HarnessContext, HarnessService, InitResult, IntakeInput, InterventionAddInput,
    InterventionFilter, KnowledgeService, MigrateResult, QueryTable, StoryAddInput,
    StoryUpdateInput, ToolRegisterInput, TraceInput,
};
use crate::domain::knowledge;
use crate::domain::{
    normalize_capability, parse_optional_integer, parse_tool_args, proof_display,
    validate_responsibility, validate_tool_kind, BacklogFilter, BacklogRecord, BoolFlag,
    ContextScoreResult, CsvList, DecisionRecord, EvidenceRecord, FrictionRecord, HarnessStats,
    DoneCheckReport, ImprovementProposal, InputType, IntakeRecord, InterventionRecord, RecapFilter,
    RecapReport, RiskLane, StatusFilter, StatusReport, StatusSection, StoryMatrixRecord,
    StoryVerifyAllResult, ToolEntry, TraceQualityTier, TraceRecord, TraceScoreResult,
    RISK_LANE_HELP,
};
use crate::infrastructure::ToolCheckResult;

#[derive(Parser, Debug)]
#[command(name = "harness-cli")]
#[command(about = "durable layer for the project harness", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create the harness database if it does not already exist.
    Init,
    /// Apply schema migrations.
    Migrate,
    /// Seed or refresh the database from existing markdown state.
    Import(ImportArgs),
    /// Record a feature intake classification.
    Intake(IntakeArgs),
    /// Add or update a story.
    Story(StoryArgs),
    /// Add a decision or run its verification.
    Decision(DecisionArgs),
    /// Add or close a backlog item.
    Backlog(BacklogArgs),
    /// Register or remove external tools.
    Tool(ToolArgs),
    /// Record a human, review, CI, or agent intervention.
    Intervention(InterventionArgs),
    /// Record an agent execution trace.
    Trace(TraceArgs),
    /// Add or list durable evidence artifacts.
    Evidence(EvidenceArgs),
    /// Score a trace against the trace quality tiers.
    ScoreTrace(ScoreTraceArgs),
    /// Score trace context reads against CONTEXT_RULES.md.
    ScoreContext { trace_id: String },
    /// Lane-aware completion gate for a story or intake (exit 1 if any check fails).
    DoneCheck(DoneCheckArgs),
    /// Run drift audit and entropy score.
    Audit,
    /// Generate improvement proposals from observed patterns.
    Propose(ProposeArgs),
    /// Query harness data.
    Query(QueryArgs),
    /// Generate or verify the repository Knowledge Index.
    Knowledge(KnowledgeArgs),
}

#[derive(Args, Debug)]
struct KnowledgeArgs {
    #[command(subcommand)]
    action: KnowledgeAction,
}

#[derive(Subcommand, Debug)]
enum KnowledgeAction {
    /// Create or refresh docs/KNOWLEDGE_INDEX.md (deterministic sections).
    Scaffold,
    /// Verify the index is present, current, and fully authored.
    Check,
}

#[derive(Args, Debug)]
#[command(after_help = RISK_LANE_HELP)]
struct IntakeArgs {
    #[arg(long = "type")]
    input_type: String,
    #[arg(long)]
    summary: String,
    #[arg(long, value_name = "tiny|normal|high-risk")]
    lane: String,
    #[arg(long)]
    flags: Option<String>,
    #[arg(long)]
    docs: Option<String>,
    #[arg(long)]
    story: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args, Debug)]
struct ImportArgs {
    #[command(subcommand)]
    source: ImportSource,
}

#[derive(Subcommand, Debug)]
enum ImportSource {
    /// Import TEST_MATRIX, decisions, and backlog markdown.
    Brownfield,
}

#[derive(Args, Debug)]
struct StoryArgs {
    #[command(subcommand)]
    action: StoryAction,
}

#[derive(Subcommand, Debug)]
enum StoryAction {
    #[command(after_help = RISK_LANE_HELP)]
    Add(StoryAddArgs),
    #[command(
        after_help = "Proof flags use numeric booleans: --unit 1 --integration 1 --e2e 0 --platform 0. Do not use yes/no."
    )]
    Update(StoryUpdateArgs),
    #[command(
        after_help = "story verify only accepts the story id. Configure proof with story add/update --verify, then record proof flags with story update."
    )]
    Verify {
        /// Story id to verify.
        id: String,
        /// Skip auto-capturing the verify log into the evidence store.
        #[arg(long = "no-capture")]
        no_capture: bool,
    },
    /// Verify every story, skipping stories without verify_command.
    VerifyAll,
}

#[derive(Args, Debug)]
struct StoryAddArgs {
    #[arg(long)]
    id: String,
    #[arg(long)]
    title: String,
    #[arg(long, value_name = "tiny|normal|high-risk")]
    lane: String,
    #[arg(long)]
    contract: Option<String>,
    #[arg(long)]
    verify: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args, Debug)]
struct StoryUpdateArgs {
    #[arg(long)]
    id: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    evidence: Option<String>,
    #[arg(long, value_name = "0|1")]
    unit: Option<String>,
    #[arg(long, value_name = "0|1")]
    integration: Option<String>,
    #[arg(long, value_name = "0|1")]
    e2e: Option<String>,
    #[arg(long, value_name = "0|1")]
    platform: Option<String>,
    #[arg(long)]
    verify: Option<String>,
    /// Live WIP resume hint: the current next step for this story.
    #[arg(long = "next-action")]
    next_action: Option<String>,
}

#[derive(Args, Debug)]
struct DecisionArgs {
    #[command(subcommand)]
    action: DecisionAction,
}

#[derive(Subcommand, Debug)]
enum DecisionAction {
    Add(DecisionAddArgs),
    Verify { id: String },
}

#[derive(Args, Debug)]
struct DecisionAddArgs {
    #[arg(long)]
    id: String,
    #[arg(long)]
    title: String,
    #[arg(long, default_value = "accepted")]
    status: String,
    #[arg(long)]
    doc: Option<String>,
    #[arg(long)]
    verify: Option<String>,
    #[arg(long)]
    predicted: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args, Debug)]
struct BacklogArgs {
    #[command(subcommand)]
    action: BacklogAction,
}

#[derive(Subcommand, Debug)]
enum BacklogAction {
    #[command(after_help = RISK_LANE_HELP)]
    Add(BacklogAddArgs),
    Close(BacklogCloseArgs),
}

#[derive(Args, Debug)]
struct BacklogAddArgs {
    #[arg(long)]
    title: String,
    #[arg(long = "while")]
    discovered_while: Option<String>,
    #[arg(long)]
    pain: Option<String>,
    #[arg(long)]
    suggestion: Option<String>,
    #[arg(long, value_name = "tiny|normal|high-risk")]
    risk: Option<String>,
    #[arg(long)]
    predicted: Option<String>,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args, Debug)]
struct BacklogCloseArgs {
    #[arg(long)]
    id: String,
    #[arg(long, default_value = "implemented")]
    status: String,
    #[arg(long)]
    outcome: Option<String>,
}

#[derive(Args, Debug)]
struct ToolArgs {
    #[command(subcommand)]
    action: ToolAction,
}

#[derive(Subcommand, Debug)]
enum ToolAction {
    Register(ToolRegisterArgs),
    /// Scan registered tools and persist present/missing/unknown status.
    Check(ToolCheckArgs),
    Remove {
        #[arg(long)]
        name: String,
    },
}

#[derive(Args, Debug)]
struct ToolRegisterArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    command: String,
    #[arg(long)]
    description: String,
    #[arg(long)]
    responsibility: String,
    #[arg(long)]
    args: Option<String>,
    #[arg(long)]
    force: bool,
    /// How the tool is reached and probed: cli, binary, mcp, skill, http.
    #[arg(long, default_value = "cli")]
    kind: String,
    /// Workflow purpose a step looks the tool up by (kebab-case).
    #[arg(long)]
    capability: Option<String>,
    /// Declarative path/URL `tool check` resolves to decide presence.
    #[arg(long)]
    scan: Option<String>,
}

#[derive(Args, Debug)]
struct ToolCheckArgs {
    /// Check one tool by name; omit to check every registered tool.
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct InterventionArgs {
    #[command(subcommand)]
    action: InterventionAction,
}

#[derive(Subcommand, Debug)]
enum InterventionAction {
    Add(InterventionAddArgs),
}

#[derive(Args, Debug)]
struct InterventionAddArgs {
    #[arg(long)]
    trace: Option<String>,
    #[arg(long)]
    story: Option<String>,
    #[arg(long = "type")]
    intervention_type: String,
    #[arg(long)]
    description: String,
    #[arg(long)]
    source: String,
    #[arg(long)]
    impact: Option<String>,
}

#[derive(Args, Debug)]
struct TraceArgs {
    #[arg(long)]
    summary: String,
    #[arg(long)]
    intake: Option<String>,
    #[arg(long)]
    story: Option<String>,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long)]
    outcome: Option<String>,
    #[arg(long)]
    duration: Option<String>,
    #[arg(long)]
    tokens: Option<String>,
    #[arg(long)]
    friction: Option<String>,
    #[arg(long)]
    actions: Option<String>,
    #[arg(long = "read")]
    files_read: Option<String>,
    #[arg(long = "changed")]
    files_changed: Option<String>,
    #[arg(long)]
    decisions: Option<String>,
    #[arg(long)]
    errors: Option<String>,
    #[arg(long)]
    notes: Option<String>,
    /// Resume hint. Required when --outcome is partial, blocked, or failed.
    #[arg(long = "next-action")]
    next_action: Option<String>,
}

#[derive(Args, Debug)]
struct EvidenceArgs {
    #[command(subcommand)]
    action: EvidenceAction,
}

#[derive(Subcommand, Debug)]
enum EvidenceAction {
    /// Hash an artifact, copy it into the gitignored store, and record a pointer.
    Add(EvidenceAddArgs),
    /// List recorded evidence pointers.
    List(EvidenceListArgs),
}

#[derive(Args, Debug)]
#[command(after_help = "Kinds: log, diff, screenshot, report, file. Sources: agent, human, ci, reviewer.")]
struct EvidenceAddArgs {
    #[arg(long = "kind")]
    kind: String,
    #[arg(long)]
    path: String,
    #[arg(long)]
    story: Option<String>,
    #[arg(long)]
    trace: Option<String>,
    #[arg(long)]
    command: Option<String>,
    #[arg(long, default_value = "agent")]
    source: String,
    #[arg(long)]
    notes: Option<String>,
}

#[derive(Args, Debug)]
struct EvidenceListArgs {
    #[arg(long)]
    story: Option<String>,
    #[arg(long)]
    trace: Option<String>,
    #[arg(long = "kind")]
    kind: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ScoreTraceArgs {
    /// Score a specific trace id. Defaults to the latest trace.
    #[arg(long)]
    id: Option<String>,
}

#[derive(Args, Debug)]
struct DoneCheckArgs {
    #[arg(long)]
    story: Option<String>,
    #[arg(long)]
    intake: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ProposeArgs {
    #[arg(long)]
    commit: bool,
}

#[derive(Args, Debug)]
struct QueryArgs {
    #[command(subcommand)]
    view: QueryView,
}

#[derive(Args, Debug)]
#[command(after_help = RISK_LANE_HELP)]
struct StatusQueryArgs {
    #[arg(long)]
    json: bool,
    /// Filter story-derived sections by lane: tiny, normal, high-risk.
    #[arg(long, value_name = "tiny|normal|high-risk")]
    lane: Option<String>,
    /// Per-section line cap (default 5).
    #[arg(long, default_value_t = 5)]
    limit: usize,
    /// Remove the per-section cap (print every row).
    #[arg(long)]
    full: bool,
}

#[derive(Args, Debug)]
struct RecapQueryArgs {
    #[arg(long)]
    story: Option<String>,
    /// Story-id prefix to aggregate (e.g. US-00 matches US-001..US-009).
    #[arg(long)]
    epic: Option<String>,
    /// Only include traces on or after this date (YYYY-MM-DD).
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct MatrixQueryArgs {
    /// Render proof flags as CLI input values, 1 and 0, instead of yes and no.
    #[arg(long)]
    numeric: bool,
}

#[derive(Args, Debug)]
struct BacklogQueryArgs {
    /// Show only proposed and accepted backlog items.
    #[arg(long, conflicts_with = "closed")]
    open: bool,
    /// Show only implemented and rejected backlog items.
    #[arg(long)]
    closed: bool,
}

#[derive(Subcommand, Debug)]
enum QueryView {
    /// Read-Model session brief: what is being / has been / needs doing.
    Status(StatusQueryArgs),
    /// Deterministic rollup of traces by story, epic prefix, or since-date.
    Recap(RecapQueryArgs),
    /// Test matrix.
    Matrix(MatrixQueryArgs),
    /// Harness improvement proposals.
    Backlog(BacklogQueryArgs),
    /// Decision records.
    Decisions,
    /// Recent intake classifications.
    Intakes,
    /// Recent traces.
    Traces,
    /// Traces with harness friction.
    Friction,
    /// Machine-readable and registered tool manifest.
    Tools(ToolsQueryArgs),
    /// Intervention records.
    Interventions(InterventionsQueryArgs),
    /// Summary counts.
    Stats,
    /// Run arbitrary SQL.
    Sql { query: Vec<String> },
}

#[derive(Args, Debug)]
struct ToolsQueryArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    summary: bool,
    #[arg(long)]
    responsibility: Option<String>,
    /// Filter to tools that provide this capability.
    #[arg(long)]
    capability: Option<String>,
    /// Filter to tools with this scanned status: present, missing, unknown.
    #[arg(long)]
    status: Option<String>,
}

#[derive(Args, Debug)]
struct InterventionsQueryArgs {
    #[arg(long)]
    trace: Option<String>,
    #[arg(long)]
    story: Option<String>,
    #[arg(long = "type")]
    intervention_type: Option<String>,
}

#[derive(Debug, Error)]
pub enum InterfaceError {
    #[error("{0}")]
    ParseHarnessValue(#[from] crate::domain::ParseHarnessValueError),
    #[error("{0}")]
    ToolValidation(#[from] crate::domain::ToolValidationError),
    #[error("{0}")]
    Infrastructure(#[from] crate::infrastructure::HarnessInfraError),
    #[error("could not determine current directory: {0}")]
    CurrentDir(std::io::Error),
    #[error("query sql requires a SQL statement")]
    EmptySql,
    #[error("knowledge check failed with {0} problem(s)")]
    KnowledgeCheckFailed(usize),
}

pub fn run(cli: Cli) -> Result<(), InterfaceError> {
    let service = HarnessService::new(resolve_context()?);

    match cli.command {
        Command::Init => print_init_result(service.init()?),
        Command::Migrate => print_migrate_result(service.migrate()?),
        Command::Import(args) => match args.source {
            ImportSource::Brownfield => {
                print_brownfield_import_result(service.import_brownfield()?)
            }
        },
        Command::Intake(args) => {
            let id = service.record_intake(IntakeInput {
                input_type: InputType::from_str(&args.input_type)?,
                summary: args.summary,
                risk_lane: RiskLane::from_str(&args.lane)?,
                risk_flags: CsvList::from_optional(args.flags),
                affected_docs: CsvList::from_optional(args.docs),
                story_id: args.story,
                notes: args.notes,
            })?;
            println!("Intake #{id} recorded.");
        }
        Command::Story(args) => match args.action {
            StoryAction::Add(args) => {
                service.add_story(StoryAddInput {
                    id: args.id.clone(),
                    title: args.title,
                    risk_lane: RiskLane::from_str(&args.lane)?,
                    contract_doc: args.contract,
                    verify_command: args.verify,
                    notes: args.notes,
                })?;
                println!("Story {} added.", args.id);
            }
            StoryAction::Update(args) => {
                service.update_story(StoryUpdateInput {
                    id: args.id.clone(),
                    status: args.status,
                    evidence: args.evidence,
                    unit: parse_optional_bool("story update: --unit", args.unit)?,
                    integration: parse_optional_bool(
                        "story update: --integration",
                        args.integration,
                    )?,
                    e2e: parse_optional_bool("story update: --e2e", args.e2e)?,
                    platform: parse_optional_bool("story update: --platform", args.platform)?,
                    verify_command: args.verify,
                    next_action: args.next_action,
                })?;
                println!("Story {} updated.", args.id);
            }
            StoryAction::Verify { id, no_capture } => {
                let result = service.verify_story(&id, !no_capture)?;
                println!("Running: {}", result.command);
                print!("{}", result.stdout);
                print!("{}", result.stderr);
                println!("Story {id} verification: {}", result.result);
                if let Some(evidence_id) = result.evidence_id {
                    println!("Captured verify log as evidence #{evidence_id}.");
                }
                if result.result == "fail" {
                    std::process::exit(1);
                }
            }
            StoryAction::VerifyAll => {
                let result = service.verify_all_stories()?;
                print_story_verify_all(&result);
                if result.failed() > 0 {
                    std::process::exit(1);
                }
            }
        },
        Command::Decision(args) => match args.action {
            DecisionAction::Add(args) => {
                service.add_decision(DecisionAddInput {
                    id: args.id.clone(),
                    title: args.title,
                    status: args.status,
                    doc_path: args.doc,
                    verify_command: args.verify,
                    predicted_impact: args.predicted,
                    notes: args.notes,
                })?;
                println!("Decision {} added.", args.id);
            }
            DecisionAction::Verify { id } => {
                let result = service.verify_decision(&id)?;
                println!("Running: {}", result.command);
                println!("Decision {id} verification: {}", result.result);
                if result.result == "fail" {
                    std::process::exit(1);
                }
            }
        },
        Command::Backlog(args) => match args.action {
            BacklogAction::Add(args) => {
                let id = service.add_backlog(BacklogAddInput {
                    title: args.title,
                    discovered_while: args.discovered_while,
                    current_pain: args.pain,
                    suggestion: args.suggestion,
                    risk: args
                        .risk
                        .map(|value| RiskLane::from_str(&value))
                        .transpose()?,
                    predicted_impact: args.predicted,
                    notes: args.notes,
                })?;
                println!("Backlog #{id} added.");
            }
            BacklogAction::Close(args) => {
                let id = parse_optional_integer("backlog close: --id", Some(args.id))?
                    .expect("value provided");
                let status = args.status;
                service.close_backlog(BacklogCloseInput {
                    id,
                    status: status.clone(),
                    actual_outcome: args.outcome,
                })?;
                println!("Backlog #{id} closed as {status}.");
            }
        },
        Command::Tool(args) => match args.action {
            ToolAction::Register(args) => {
                let kind = validate_tool_kind(&args.kind)?;
                let capability = args
                    .capability
                    .as_deref()
                    .map(normalize_capability)
                    .transpose()?;
                service.register_tool(ToolRegisterInput {
                    name: args.name.clone(),
                    command: args.command,
                    description: args.description,
                    responsibility: validate_responsibility(&args.responsibility)?,
                    args: parse_tool_args(args.args)?,
                    force: args.force,
                    kind,
                    capability,
                    scan_target: args.scan,
                })?;
                println!("Tool {} registered.", args.name);
            }
            ToolAction::Check(args) => {
                let results = service.check_tools(args.name)?;
                if args.json {
                    print_tool_check_json(&results);
                } else {
                    print_tool_check_summary(&results);
                }
            }
            ToolAction::Remove { name } => {
                service.remove_tool(&name)?;
                println!("Tool {name} removed.");
            }
        },
        Command::Intervention(args) => match args.action {
            InterventionAction::Add(args) => {
                let id = service.add_intervention(InterventionAddInput {
                    trace_id: parse_optional_integer("intervention add: --trace", args.trace)?,
                    story_id: args.story,
                    intervention_type: args.intervention_type,
                    description: args.description,
                    source: args.source,
                    impact: args.impact,
                })?;
                println!("Intervention #{id} recorded.");
            }
        },
        Command::Trace(args) => {
            let story_id = args.story.clone();
            let id = service.record_trace(TraceInput {
                task_summary: args.summary,
                intake_id: parse_optional_integer("trace: --intake", args.intake)?,
                story_id: args.story,
                agent: args.agent,
                outcome: args.outcome,
                duration_seconds: parse_optional_integer("trace: --duration", args.duration)?,
                token_estimate: parse_optional_integer("trace: --tokens", args.tokens)?,
                friction: args.friction,
                notes: args.notes,
                next_action: args.next_action,
                actions: CsvList::from_optional(args.actions),
                files_read: CsvList::from_optional(args.files_read),
                files_changed: CsvList::from_optional(args.files_changed),
                decisions: CsvList::from_optional(args.decisions),
                errors: CsvList::from_optional(args.errors),
            })?;
            println!("Trace #{id} recorded.");
            let result = service.score_trace(Some(id))?;
            print_trace_score(&result, false);
            println!("Reminder: Record any human corrections with: harness-cli intervention add");
            if let Some(story_id) = story_id {
                print_story_verify_warning(&service, &story_id)?;
            }
        }
        Command::Evidence(args) => match args.action {
            EvidenceAction::Add(args) => {
                let trace_id = parse_optional_integer("evidence add: --trace", args.trace)?;
                let result = service.add_evidence(EvidenceAddInput {
                    kind: args.kind,
                    path: args.path,
                    story_id: args.story,
                    trace_id,
                    command: args.command,
                    source: args.source,
                    result: None,
                    notes: args.notes,
                })?;
                let verb = if result.deduped {
                    "Evidence already recorded (deduped)"
                } else {
                    "Evidence recorded"
                };
                println!("{verb} #{}", result.id);
                println!("  path:   {}", result.path);
                println!("  sha256: {}", result.sha256);
                println!("  bytes:  {}", result.bytes);
            }
            EvidenceAction::List(args) => {
                let trace_id = parse_optional_integer("evidence list: --trace", args.trace)?;
                let records = service.list_evidence(EvidenceFilter {
                    story_id: args.story,
                    trace_id,
                    kind: args.kind,
                })?;
                if args.json {
                    print_evidence_json(&records);
                } else {
                    print_evidence_table(&records);
                }
            }
        },
        Command::ScoreTrace(args) => {
            let id = parse_optional_integer("score-trace: --id", args.id)?;
            let result = service.score_trace(id)?;
            print_trace_score(&result, id.is_none());
            if !result.meets_requirement {
                std::process::exit(1);
            }
        }
        Command::ScoreContext { trace_id } => {
            let id = parse_optional_integer("score-context: trace-id", Some(trace_id))?
                .expect("value provided");
            print_context_score(&service.score_context(id)?);
        }
        Command::Knowledge(args) => {
            let service = KnowledgeService::new(resolve_repo_root()?);
            match args.action {
                KnowledgeAction::Scaffold => {
                    let result = service.scaffold()?;
                    let verb = if result.created {
                        "Created"
                    } else {
                        "Refreshed"
                    };
                    println!("{verb} {}", result.path.display());
                    println!(
                        "Fill the Purpose and Key Concepts blocks, then run: harness-cli knowledge check"
                    );
                }
                KnowledgeAction::Check => {
                    let problems = service.check()?;
                    if problems.is_empty() {
                        println!("Knowledge Index OK: {}", knowledge::INDEX_PATH);
                    } else {
                        eprintln!("Knowledge Index has {} problem(s):", problems.len());
                        for problem in &problems {
                            eprintln!("  - {problem}");
                        }
                        return Err(InterfaceError::KnowledgeCheckFailed(problems.len()));
                    }
                }
            }
        }
        Command::DoneCheck(args) => {
            let intake = parse_optional_integer("done-check: --intake", args.intake)?;
            let report = service.done_check(args.story, intake)?;
            if args.json {
                print_done_check_json(&report);
            } else {
                print_done_check_text(&report);
            }
            if !report.passed() {
                std::process::exit(1);
            }
        }
        Command::Audit => print_audit(&service.audit()?),
        Command::Propose(args) => print_proposals(&service.propose(args.commit)?),
        Command::Query(args) => match args.view {
            QueryView::Status(args) => {
                let lane = args
                    .lane
                    .map(|value| RiskLane::from_str(&value))
                    .transpose()?
                    .map(|lane| lane.as_db_value().to_owned());
                let filter = StatusFilter {
                    lane,
                    limit: if args.full { None } else { Some(args.limit) },
                };
                let report = service.query_status(filter)?;
                if args.json {
                    print_status_json(&report);
                } else {
                    print_status_text(&report);
                }
            }
            QueryView::Recap(args) => {
                let report = service.query_recap(RecapFilter {
                    story_id: args.story,
                    epic_prefix: args.epic,
                    since: args.since,
                })?;
                if args.json {
                    print_recap_json(&report);
                } else {
                    print_recap_text(&report);
                }
            }
            QueryView::Matrix(args) => print_matrix(&service.query_matrix()?, args.numeric),
            QueryView::Backlog(args) => {
                print_backlog(&service.query_backlog(backlog_filter(&args))?)
            }
            QueryView::Decisions => print_decisions(&service.query_decisions()?),
            QueryView::Intakes => print_intakes(&service.query_intakes()?),
            QueryView::Traces => print_traces(&service.query_traces()?),
            QueryView::Friction => print_friction(&service.query_friction()?),
            QueryView::Tools(args) => {
                let responsibility = args
                    .responsibility
                    .map(|value| validate_responsibility(&value))
                    .transpose()?;
                let capability = args
                    .capability
                    .as_deref()
                    .map(normalize_capability)
                    .transpose()?;
                let mut tools = service.query_tools(responsibility, capability)?;
                if let Some(status) = args.status.as_deref() {
                    let normalized = status.trim().to_lowercase();
                    tools.retain(|tool| tool.status == normalized);
                }
                if args.json {
                    print_tools_json(&tools);
                } else {
                    print_tools_summary(&tools);
                }
            }
            QueryView::Interventions(args) => {
                let trace_id = parse_optional_integer("query interventions: --trace", args.trace)?;
                print_interventions(&service.query_interventions(InterventionFilter {
                    trace_id,
                    story_id: args.story,
                    intervention_type: args.intervention_type,
                })?);
            }
            QueryView::Stats => print_stats(&service.query_stats()?),
            QueryView::Sql { query } => {
                if query.is_empty() {
                    return Err(InterfaceError::EmptySql);
                }
                print_query_table(&service.query_sql(&query.join(" "))?);
            }
        },
    }

    Ok(())
}

fn print_trace_score(result: &TraceScoreResult, latest: bool) {
    if latest {
        println!("Trace #{} (latest):", result.trace_id);
    } else {
        println!("Trace #{}:", result.trace_id);
    }
    println!(
        "  Tier achieved: {} ({}/3)",
        result.achieved.label(),
        result.achieved.score()
    );

    match (&result.risk_lane, result.required) {
        (Some(lane), Some(required)) => {
            println!(
                "  Lane: {} -> required tier: {} ({}/3)",
                lane,
                required.label(),
                required.score()
            );
            if result.meets_requirement {
                println!("  MEETS REQUIREMENT");
            } else {
                println!("  BELOW REQUIREMENT");
            }
        }
        _ => {
            println!("  Lane: unknown (no linked intake)");
        }
    }

    print_missing_fields(
        "minimal",
        TraceQualityTier::Minimal,
        &result.missing_minimal,
    );
    print_missing_fields(
        "standard",
        TraceQualityTier::Standard,
        &result.missing_standard,
    );
    print_missing_fields(
        "detailed",
        TraceQualityTier::Detailed,
        &result.missing_detailed,
    );
}

fn print_story_verify_all(result: &StoryVerifyAllResult) {
    for item in &result.items {
        match item.result.as_str() {
            "skipped" => println!("Story {}: skipped (no verify_command)", item.id),
            status => {
                println!("Story {}: {status}", item.id);
                if !item.stdout.is_empty() {
                    print!("{}", item.stdout);
                }
                if !item.stderr.is_empty() {
                    print!("{}", item.stderr);
                }
            }
        }
    }
    println!(
        "{} stories verified: {} passed, {} failed, {} skipped (no verify_command)",
        result.items.len(),
        result.passed(),
        result.failed(),
        result.skipped()
    );
}

fn print_context_score(result: &ContextScoreResult) {
    println!(
        "Trace #{} | Lane: {} | Phase: {}",
        result.trace_id, result.lane, result.phase
    );
    println!();
    let must_met = result.must.iter().filter(|item| item.met).count();
    println!("Must-read compliance: {must_met}/{}", result.must.len());
    for item in &result.must {
        println!(
            "  {} {} ({})",
            if item.met { "OK" } else { "MISSING" },
            item.label,
            item.target
        );
    }
    let should_met = result.should.iter().filter(|item| item.met).count();
    println!(
        "Should-read compliance: {should_met}/{}",
        result.should.len()
    );
    for item in &result.should {
        println!(
            "  {} {} ({})",
            if item.met { "OK" } else { "MISSING" },
            item.label,
            item.target
        );
    }
    println!("Over-reading: {} item(s)", result.over_read.len());
    for item in &result.over_read {
        println!("  - {item}");
    }
}

fn print_done_check_text(report: &DoneCheckReport) {
    println!(
        "DONE-CHECK  {}  (lane: {})",
        report.target, report.lane
    );
    for check in &report.checks {
        let mark = if check.passed { "✔" } else { "✘" };
        println!("  {mark} {} — {}", check.label, check.detail);
    }
    if report.passed() {
        println!("RESULT: PASS");
    } else {
        println!("RESULT: FAIL (not done)");
    }
}

fn print_done_check_json(report: &DoneCheckReport) {
    println!("{{");
    println!("  \"target\": \"{}\",", json_escape(&report.target));
    println!("  \"lane\": \"{}\",", json_escape(&report.lane));
    println!("  \"passed\": {},", report.passed());
    println!("  \"checks\": [");
    for (index, check) in report.checks.iter().enumerate() {
        let comma = json_comma(index, report.checks.len());
        println!(
            "    {{\"label\": \"{}\", \"passed\": {}, \"detail\": \"{}\"}}{comma}",
            json_escape(&check.label),
            check.passed,
            json_escape(&check.detail)
        );
    }
    println!("  ]");
    println!("}}");
}

fn print_audit(result: &crate::domain::AuditResult) {
    println!("=== Harness Drift Audit ===");
    print_audit_category(
        "Orphaned stories (planned/in-progress, no traces)",
        &result.orphaned_stories,
    );
    print_audit_category("Unverified stories", &result.unverified_stories);
    print_audit_category("Unverified decisions", &result.unverified_decisions);
    print_audit_category(
        "Open backlog without outcomes",
        &result.backlog_without_outcomes,
    );
    print_audit_category("Stale stories", &result.stale_stories);
    print_audit_category("Broken tools", &result.broken_tools);
    println!(
        "Entropy score: {}/100 (lower is better)",
        result.entropy_score()
    );
}

fn print_audit_category(label: &str, findings: &[crate::domain::AuditFinding]) {
    println!();
    println!("{label}: {}", findings.len());
    for finding in findings {
        println!("  - {}: {}", finding.id, finding.title);
    }
}

fn print_proposals(proposals: &[ImprovementProposal]) {
    println!("=== Improvement Proposals ===");
    if proposals.is_empty() {
        println!("No proposals generated.");
        return;
    }
    for (index, proposal) in proposals.iter().enumerate() {
        println!();
        println!(
            "Proposal {} ({} confidence):",
            index + 1,
            proposal.confidence
        );
        println!("  Title: {}", proposal.title);
        println!("  Component: {}", proposal.component);
        println!("  Evidence: {}", proposal.evidence);
        println!("  Predicted impact: {}", proposal.predicted_impact);
        println!("  Risk: {}", proposal.risk);
        println!("  Suggested action: {}", proposal.suggested_action);
        println!("  Validation: {}", proposal.validation_plan);
        if let Some(id) = proposal.committed_backlog_id {
            println!("  Created backlog item #{id}");
        }
    }
    println!();
    println!(
        "{} proposals generated. Use --commit to create backlog items.",
        proposals.len()
    );
}

fn print_story_verify_warning(
    service: &HarnessService,
    story_id: &str,
) -> Result<(), InterfaceError> {
    let status = service.story_verify_status(story_id)?;
    let has_command = status
        .verify_command
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    if has_command && status.last_verified_result.as_deref() != Some("pass") {
        println!();
        println!(
            "Warning: Story {} has verify_command but verification has not passed.",
            status.id
        );
        println!("Run: harness-cli story verify {}", status.id);
    }
    Ok(())
}

fn print_missing_fields(label: &str, tier: TraceQualityTier, fields: &[String]) {
    if fields.is_empty() {
        return;
    }
    println!();
    println!("  Missing for {label}:");
    for field in fields {
        println!("    - {field}");
    }
    if tier == TraceQualityTier::Detailed {
        println!();
    }
}

fn backlog_filter(args: &BacklogQueryArgs) -> BacklogFilter {
    if args.open {
        BacklogFilter::Open
    } else if args.closed {
        BacklogFilter::Closed
    } else {
        BacklogFilter::All
    }
}

fn print_brownfield_import_result(result: BrownfieldImportResult) {
    println!("Brownfield import complete.");
    println!("Stories imported or updated: {}", result.stories);
    println!("Decisions imported or updated: {}", result.decisions);
    println!("Backlog items discovered: {}", result.backlog_items);
}

fn parse_optional_bool(
    label: &str,
    value: Option<String>,
) -> Result<Option<BoolFlag>, InterfaceError> {
    value
        .map(|inner| BoolFlag::parse(label, &inner))
        .transpose()
        .map_err(InterfaceError::from)
}

fn print_init_result(result: InitResult) {
    match result {
        InitResult::Created { db_path } => {
            println!("Creating harness database at {}", db_path.display());
            println!("Schema applied.");
        }
        InitResult::Existing { db_path, version } => {
            println!("Database already exists at {}", db_path.display());
            println!("Current schema version: {version}");
        }
        InitResult::MigratedExisting { db_path } => {
            println!("Database already exists at {}", db_path.display());
            println!("No schema version found. Applying schema.");
            println!("Schema applied.");
        }
    }
}

fn print_migrate_result(result: MigrateResult) {
    println!("Current schema version: {}", result.current_version);
    if result.applied.is_empty() {
        println!("Already up to date.");
    } else {
        for version in &result.applied {
            println!("Applying migration {version}...");
        }
        println!("Applied {} migration(s).", result.applied.len());
    }
}

fn resolve_repo_root() -> Result<PathBuf, InterfaceError> {
    match env::var_os("HARNESS_REPO_ROOT") {
        Some(path) => Ok(PathBuf::from(path)),
        None => env::current_dir().map_err(InterfaceError::CurrentDir),
    }
}

fn resolve_context() -> Result<HarnessContext, InterfaceError> {
    let repo_root = resolve_repo_root()?;
    let db_path = env::var_os("HARNESS_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("harness.db"));

    let schema_dir = repo_root.join("scripts/schema");

    Ok(HarnessContext {
        repo_root,
        db_path,
        schema_dir,
    })
}

fn status_header<T>(label: &str, section: &StatusSection<T>) {
    println!("▌ {label}   {}", section.total);
    if section.items.is_empty() {
        println!("  • (none)");
    }
}

fn status_hidden<T>(section: &StatusSection<T>) {
    let hidden = section.hidden();
    if hidden > 0 {
        println!("  (+{hidden} nữa — dùng --full)");
    }
}

fn print_status_text(report: &StatusReport) {
    println!(
        "HARNESS STATUS  (drift: {}/100 · {} group(s))",
        report.entropy_score, report.drift_groups
    );

    println!();
    status_header("ĐANG LÀM (in_progress)", &report.active);
    for story in &report.active.items {
        let next = story
            .next_action
            .as_deref()
            .map(|value| format!("  → next: {value}"))
            .unwrap_or_default();
        println!("  • {:<8} [{}] {}{}", story.id, story.lane, story.title, next);
    }
    status_hidden(&report.active);

    println!();
    status_header("CẦN PROOF (implemented, chưa pass)", &report.needs_proof);
    for gap in &report.needs_proof.items {
        println!(
            "  • {:<8} {}  verify={} unit={} integ={} e2e={} plat={}",
            gap.id,
            gap.title,
            gap.verify_result.as_deref().unwrap_or("none"),
            gap.unit,
            gap.integration,
            gap.e2e,
            gap.platform,
        );
    }
    status_hidden(&report.needs_proof);

    println!();
    status_header("RESUME (partial/blocked/failed)", &report.resume);
    for item in &report.resume.items {
        let story = item.story_id.as_deref().unwrap_or("-");
        let next = item.next_action.as_deref().unwrap_or("-");
        println!(
            "  • trace#{:<4} {:<8} {}  → next: {}",
            item.trace_id, item.outcome, story, next
        );
    }
    status_hidden(&report.resume);

    println!();
    status_header("BACKLOG MỞ (high-risk trước)", &report.backlog);
    for item in &report.backlog.items {
        let risk = item.risk.as_deref().unwrap_or("-");
        let pred = item
            .predicted
            .as_deref()
            .map(|value| format!("  pred: {value}"))
            .unwrap_or_default();
        println!("  • #{:<4} [{}] {}{}", item.id, risk, item.title, pred);
    }
    status_hidden(&report.backlog);

    println!();
    status_header("INTERVENTION gần đây", &report.interventions);
    for item in &report.interventions.items {
        let trace = item
            .trace_id
            .map(|value| format!("trace#{value}"))
            .unwrap_or_else(|| "-".to_owned());
        println!(
            "  • #{:<4} {} ({}) on {}",
            item.id, item.intervention_type, item.source, trace
        );
    }
    status_hidden(&report.interventions);

    println!();
    status_header("HOẠT ĐỘNG GẦN NHẤT", &report.recent);
    for item in &report.recent.items {
        let outcome = item.outcome.as_deref().unwrap_or("-");
        println!("  • trace#{:<4} {:<9} {}", item.trace_id, outcome, item.task_summary);
    }
    status_hidden(&report.recent);
}

fn print_status_json(report: &StatusReport) {
    println!("{{");
    println!("  \"entropy_score\": {},", report.entropy_score);
    println!("  \"drift_groups\": {},", report.drift_groups);

    println!("  \"active\": {{ \"total\": {}, \"items\": [", report.active.total);
    for (index, story) in report.active.items.iter().enumerate() {
        let comma = json_comma(index, report.active.items.len());
        println!(
            "    {{\"id\": \"{}\", \"lane\": \"{}\", \"title\": \"{}\", \"next_action\": {}}}{comma}",
            json_escape(&story.id),
            json_escape(&story.lane),
            json_escape(&story.title),
            json_optional(story.next_action.as_deref())
        );
    }
    println!("  ]}},");

    println!("  \"needs_proof\": {{ \"total\": {}, \"items\": [", report.needs_proof.total);
    for (index, gap) in report.needs_proof.items.iter().enumerate() {
        let comma = json_comma(index, report.needs_proof.items.len());
        println!(
            "    {{\"id\": \"{}\", \"verify_result\": {}, \"unit\": {}, \"integration\": {}, \"e2e\": {}, \"platform\": {}}}{comma}",
            json_escape(&gap.id),
            json_optional(gap.verify_result.as_deref()),
            gap.unit, gap.integration, gap.e2e, gap.platform
        );
    }
    println!("  ]}},");

    println!("  \"resume\": {{ \"total\": {}, \"items\": [", report.resume.total);
    for (index, item) in report.resume.items.iter().enumerate() {
        let comma = json_comma(index, report.resume.items.len());
        println!(
            "    {{\"trace_id\": {}, \"story_id\": {}, \"outcome\": \"{}\", \"next_action\": {}}}{comma}",
            item.trace_id,
            json_optional(item.story_id.as_deref()),
            json_escape(&item.outcome),
            json_optional(item.next_action.as_deref())
        );
    }
    println!("  ]}},");

    println!("  \"backlog\": {{ \"total\": {}, \"items\": [", report.backlog.total);
    for (index, item) in report.backlog.items.iter().enumerate() {
        let comma = json_comma(index, report.backlog.items.len());
        println!(
            "    {{\"id\": {}, \"risk\": {}, \"title\": \"{}\", \"predicted\": {}}}{comma}",
            item.id,
            json_optional(item.risk.as_deref()),
            json_escape(&item.title),
            json_optional(item.predicted.as_deref())
        );
    }
    println!("  ]}},");

    println!(
        "  \"interventions\": {{ \"total\": {}, \"items\": [",
        report.interventions.total
    );
    for (index, item) in report.interventions.items.iter().enumerate() {
        let comma = json_comma(index, report.interventions.items.len());
        println!(
            "    {{\"id\": {}, \"type\": \"{}\", \"source\": \"{}\", \"trace_id\": {}}}{comma}",
            item.id,
            json_escape(&item.intervention_type),
            json_escape(&item.source),
            item.trace_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned())
        );
    }
    println!("  ]}},");

    println!("  \"recent\": {{ \"total\": {}, \"items\": [", report.recent.total);
    for (index, item) in report.recent.items.iter().enumerate() {
        let comma = json_comma(index, report.recent.items.len());
        println!(
            "    {{\"trace_id\": {}, \"outcome\": {}, \"task_summary\": \"{}\"}}{comma}",
            item.trace_id,
            json_optional(item.outcome.as_deref()),
            json_escape(&item.task_summary)
        );
    }
    println!("  ]}}");
    println!("}}");
}

fn recap_counts_line(counts: &[crate::domain::RecapCount]) -> String {
    if counts.is_empty() {
        return "(none)".to_owned();
    }
    counts
        .iter()
        .map(|item| format!("{} ({})", item.key, item.count))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn print_recap_text(report: &RecapReport) {
    let window = match (report.first_at.as_deref(), report.last_at.as_deref()) {
        (Some(first), Some(last)) => format!("{first} → {last}, "),
        _ => String::new(),
    };
    println!(
        "RECAP  {}  ({}{} traces)",
        report.scope, window, report.trace_count
    );
    println!();
    println!(
        "Outcome:      completed {} · partial {} · blocked {} · failed {}",
        report.completed, report.partial, report.blocked, report.failed
    );
    println!("Files đụng:   {}", recap_counts_line(&report.files));
    println!("Friction:     {}", recap_counts_line(&report.friction));
    let decisions = if report.decisions.is_empty() {
        "(none)".to_owned()
    } else {
        report.decisions.join(", ")
    };
    println!("Decisions:    {decisions}");
    println!("Intervention: {}", recap_counts_line(&report.interventions));
}

fn print_recap_counts_json(counts: &[crate::domain::RecapCount]) {
    print!("[");
    for (index, item) in counts.iter().enumerate() {
        let comma = json_comma(index, counts.len());
        print!(
            "{{\"key\": \"{}\", \"count\": {}}}{comma}",
            json_escape(&item.key),
            item.count
        );
    }
    print!("]");
}

fn print_recap_json(report: &RecapReport) {
    println!("{{");
    println!("  \"scope\": \"{}\",", json_escape(&report.scope));
    println!("  \"first_at\": {},", json_optional(report.first_at.as_deref()));
    println!("  \"last_at\": {},", json_optional(report.last_at.as_deref()));
    println!("  \"trace_count\": {},", report.trace_count);
    println!(
        "  \"outcome\": {{\"completed\": {}, \"partial\": {}, \"blocked\": {}, \"failed\": {}}},",
        report.completed, report.partial, report.blocked, report.failed
    );
    print!("  \"files\": ");
    print_recap_counts_json(&report.files);
    println!(",");
    print!("  \"friction\": ");
    print_recap_counts_json(&report.friction);
    println!(",");
    print!("  \"decisions\": [");
    for (index, decision) in report.decisions.iter().enumerate() {
        let comma = json_comma(index, report.decisions.len());
        print!("\"{}\"{comma}", json_escape(decision));
    }
    println!("],");
    print!("  \"interventions\": ");
    print_recap_counts_json(&report.interventions);
    println!();
    println!("}}");
}

fn json_comma(index: usize, len: usize) -> &'static str {
    if index + 1 == len {
        ""
    } else {
        ","
    }
}

fn print_matrix(records: &[StoryMatrixRecord], numeric: bool) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.clone(),
                record.title.clone(),
                record.status.clone(),
                proof_display(record.unit, numeric),
                proof_display(record.integration, numeric),
                proof_display(record.e2e, numeric),
                proof_display(record.platform, numeric),
                record.evidence.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "id", "title", "status", "unit", "integ", "e2e", "plat", "evidence",
        ],
        &rows,
    );
}

fn print_backlog(records: &[BacklogRecord]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.to_string(),
                record.title.clone(),
                record.status.clone(),
                record.risk.clone().unwrap_or_default(),
                record.predicted_impact.clone().unwrap_or_default(),
                record.actual_outcome.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "id",
            "title",
            "status",
            "risk",
            "predicted_impact",
            "actual_outcome",
        ],
        &rows,
    );
}

fn print_decisions(records: &[DecisionRecord]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.clone(),
                record.title.clone(),
                record.status.clone(),
                record.last_verified_at.clone().unwrap_or_default(),
                record.last_verified_result.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "id",
            "title",
            "status",
            "last_verified_at",
            "last_verified_result",
        ],
        &rows,
    );
}

fn print_intakes(records: &[IntakeRecord]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.to_string(),
                record.created_at.clone(),
                record.input_type.clone(),
                record.risk_lane.clone(),
                record.summary.clone(),
            ]
        })
        .collect::<Vec<_>>();

    print_table(
        &["id", "created_at", "input_type", "risk_lane", "summary"],
        &rows,
    );
}

fn print_traces(records: &[TraceRecord]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.to_string(),
                record.created_at.clone(),
                record.outcome.clone().unwrap_or_default(),
                record.task_summary.clone(),
                record.harness_friction.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "id",
            "created_at",
            "outcome",
            "task_summary",
            "harness_friction",
        ],
        &rows,
    );
}

fn print_friction(records: &[FrictionRecord]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.to_string(),
                record.created_at.clone(),
                record.risk_lane.clone().unwrap_or_else(|| "-".to_owned()),
                record.input_type.clone().unwrap_or_else(|| "-".to_owned()),
                record.task_summary.clone(),
                record.harness_friction.clone(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "id",
            "created_at",
            "risk_lane",
            "input_type",
            "task_summary",
            "harness_friction",
        ],
        &rows,
    );
}

fn print_tools_summary(records: &[ToolEntry]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.name.clone(),
                record.kind.clone(),
                record.capability.clone().unwrap_or_else(|| "-".to_owned()),
                record.responsibility.clone(),
                record.status.clone(),
                record.source.clone(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "name",
            "kind",
            "capability",
            "responsibility",
            "status",
            "source",
        ],
        &rows,
    );
}

fn print_tools_json(records: &[ToolEntry]) {
    println!("[");
    for (index, record) in records.iter().enumerate() {
        let comma = if index + 1 == records.len() { "" } else { "," };
        println!("  {{");
        println!("    \"provider\": \"{}\",", json_escape(&record.provider));
        println!("    \"name\": \"{}\",", json_escape(&record.name));
        println!("    \"command\": \"{}\",", json_escape(&record.command));
        println!(
            "    \"description\": \"{}\",",
            json_escape(&record.description)
        );
        println!("    \"args\": [");
        for (arg_index, arg) in record.args.iter().enumerate() {
            let arg_comma = if arg_index + 1 == record.args.len() {
                ""
            } else {
                ","
            };
            println!(
                "      {{\"name\":\"{}\",\"type\":\"{}\",\"required\":{},\"help\":\"{}\"}}{}",
                json_escape(&arg.name),
                json_escape(&arg.arg_type),
                arg.required,
                json_escape(arg.help.as_deref().unwrap_or("")),
                arg_comma
            );
        }
        println!("    ],");
        println!(
            "    \"responsibility\": \"{}\",",
            json_escape(&record.responsibility)
        );
        println!("    \"source\": \"{}\",", json_escape(&record.source));
        println!("    \"since\": \"{}\",", json_escape(&record.since));
        println!("    \"kind\": \"{}\",", json_escape(&record.kind));
        println!(
            "    \"capability\": {},",
            json_optional(record.capability.as_deref())
        );
        println!(
            "    \"scan_target\": {},",
            json_optional(record.scan_target.as_deref())
        );
        println!("    \"status\": \"{}\",", json_escape(&record.status));
        println!(
            "    \"checked_at\": {}",
            json_optional(record.checked_at.as_deref())
        );
        println!("  }}{comma}");
    }
    println!("]");
}

fn print_tool_check_summary(records: &[ToolCheckResult]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.name.clone(),
                record.kind.clone(),
                record.capability.clone().unwrap_or_else(|| "-".to_owned()),
                record.status.clone(),
                record.detail.clone(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(&["name", "kind", "capability", "status", "detail"], &rows);
}

fn print_tool_check_json(records: &[ToolCheckResult]) {
    println!("[");
    for (index, record) in records.iter().enumerate() {
        let comma = if index + 1 == records.len() { "" } else { "," };
        println!("  {{");
        println!("    \"name\": \"{}\",", json_escape(&record.name));
        println!("    \"kind\": \"{}\",", json_escape(&record.kind));
        println!(
            "    \"capability\": {},",
            json_optional(record.capability.as_deref())
        );
        println!("    \"status\": \"{}\",", json_escape(&record.status));
        println!("    \"detail\": \"{}\"", json_escape(&record.detail));
        println!("  }}{comma}");
    }
    println!("]");
}

fn json_optional(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", json_escape(value)),
        None => "null".to_owned(),
    }
}

fn print_interventions(records: &[InterventionRecord]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.to_string(),
                record.created_at.clone(),
                record
                    .trace_id
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                record.story_id.clone().unwrap_or_default(),
                record.intervention_type.clone(),
                record.source.clone(),
                record.description.clone(),
                record.impact.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "id",
            "created_at",
            "trace",
            "story",
            "type",
            "source",
            "description",
            "impact",
        ],
        &rows,
    );
}

fn print_evidence_table(records: &[EvidenceRecord]) {
    let rows = records
        .iter()
        .map(|record| {
            vec![
                record.id.to_string(),
                record.created_at.clone(),
                record.story_id.clone().unwrap_or_default(),
                record
                    .trace_id
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                record.kind.clone(),
                record.result.clone().unwrap_or_default(),
                record.bytes.map(|value| value.to_string()).unwrap_or_default(),
                format!("{}…", &record.sha256[..12.min(record.sha256.len())]),
                record.path.clone(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "id", "created_at", "story", "trace", "kind", "result", "bytes", "sha256", "path",
        ],
        &rows,
    );
}

fn print_evidence_json(records: &[EvidenceRecord]) {
    println!("[");
    for (index, record) in records.iter().enumerate() {
        let comma = if index + 1 == records.len() { "" } else { "," };
        println!("  {{");
        println!("    \"id\": {},", record.id);
        println!("    \"created_at\": \"{}\",", json_escape(&record.created_at));
        println!("    \"story_id\": {},", json_optional(record.story_id.as_deref()));
        println!(
            "    \"trace_id\": {},",
            record
                .trace_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned())
        );
        println!("    \"kind\": \"{}\",", json_escape(&record.kind));
        println!("    \"path\": \"{}\",", json_escape(&record.path));
        println!("    \"sha256\": \"{}\",", json_escape(&record.sha256));
        println!(
            "    \"bytes\": {},",
            record
                .bytes
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned())
        );
        println!("    \"digest\": {},", json_optional(record.digest.as_deref()));
        println!("    \"command\": {},", json_optional(record.command.as_deref()));
        println!("    \"result\": {},", json_optional(record.result.as_deref()));
        println!("    \"source\": \"{}\",", json_escape(&record.source));
        println!("    \"notes\": {}", json_optional(record.notes.as_deref()));
        println!("  }}{comma}");
    }
    println!("]");
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn print_stats(stats: &HarnessStats) {
    println!("=== Harness Stats ===");
    print_table(
        &["intakes", "stories", "decisions", "backlog_items", "traces"],
        &[vec![
            stats.intakes.to_string(),
            stats.stories.to_string(),
            stats.decisions.to_string(),
            stats.backlog_items.to_string(),
            stats.traces.to_string(),
        ]],
    );
}

fn print_query_table(table: &QueryTable) {
    let headers = table.headers.iter().map(String::as_str).collect::<Vec<_>>();
    print_table(&headers, &table.rows);
}

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let widths = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .map(String::len)
                .chain(std::iter::once(header.len()))
                .max()
                .unwrap_or(header.len())
        })
        .collect::<Vec<_>>();

    print_row(
        &headers
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
        &widths,
    );
    print_row(
        &widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>(),
        &widths,
    );
    for row in rows {
        print_row(row, &widths);
    }
}

fn print_row(values: &[String], widths: &[usize]) {
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            print!("  ");
        }
        let value = values.get(index).map(String::as_str).unwrap_or("");
        print!("{value:<width$}");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn knowledge_commands_remain_exposed() {
        let mut command = Cli::command();
        let knowledge = command.find_subcommand_mut("knowledge").unwrap();
        assert!(knowledge.find_subcommand_mut("scaffold").is_some());
        assert!(knowledge.find_subcommand_mut("check").is_some());
    }

    #[test]
    fn story_help_documents_proof_command_shape() {
        let mut command = Cli::command();
        let story = command.find_subcommand_mut("story").unwrap();

        let update_help = story
            .find_subcommand_mut("update")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(update_help.contains("--unit <0|1>"));
        assert!(update_help.contains("--integration <0|1>"));
        assert!(update_help.contains("Proof flags use numeric booleans"));

        let verify_help = story
            .find_subcommand_mut("verify")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(verify_help.contains("story verify only accepts the story id"));
        assert!(verify_help.contains("Configure proof with story add/update --verify"));
    }

    #[test]
    fn command_help_documents_lane_values_and_version() {
        let mut command = Cli::command();
        assert!(command.render_long_help().to_string().contains("--version"));

        let intake_help = command
            .find_subcommand_mut("intake")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(intake_help.contains("--lane <tiny|normal|high-risk>"));
        assert!(intake_help.contains("Use tiny instead of low"));

        let story_add_help = command
            .find_subcommand_mut("story")
            .unwrap()
            .find_subcommand_mut("add")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(story_add_help.contains("--lane <tiny|normal|high-risk>"));

        let backlog_add_help = command
            .find_subcommand_mut("backlog")
            .unwrap()
            .find_subcommand_mut("add")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(backlog_add_help.contains("--risk <tiny|normal|high-risk>"));
        assert!(backlog_add_help.contains("Accepted lanes"));

        let matrix_help = command
            .find_subcommand_mut("query")
            .unwrap()
            .find_subcommand_mut("matrix")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(matrix_help.contains("--numeric"));
    }
}
