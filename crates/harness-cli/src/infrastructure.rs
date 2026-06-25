use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use rusqlite::{params, types::ValueRef, Connection, OptionalExtension};
use thiserror::Error;

use crate::application::{
    BacklogAddInput, BacklogCloseInput, BrownfieldImportResult, DecisionAddInput,
    DecisionVerifyResult, EvidenceAddInput, EvidenceAddResult, EvidenceFilter, HarnessContext,
    InitResult, IntakeInput, InterventionAddInput, InterventionFilter, MigrateResult, QueryTable,
    StoryAddInput, StoryUpdateInput, StoryVerifyResult, ToolRegisterInput, TraceInput,
};
use crate::domain::knowledge::{self, KnowledgeInputs, RunCommand, TopLevelEntry};
use crate::domain::{
    compiled_tool_registry, evidence, jsonish_list, normalize_token, score_context, score_trace,
    validate_tool_description, AuditFinding, AuditResult, BacklogFilter, BacklogRecord,
    ContextScoreResult, ContextScoreSource, DecisionRecord, DoneCheckItem, DoneCheckReport,
    EvidenceRecord, FrictionRecord,
    HarnessStats, ImprovementProposal, IntakeRecord, InterventionRecord, RecapCount, RecapFilter,
    RecapReport, RiskLane, StatusActivity, StatusBacklogItem, StatusFilter, StatusInterventionItem,
    StatusProofGap, StatusReport, StatusResume, StatusSection, StatusStory, StoryMatrixRecord,
    StoryVerifyAllItem, StoryVerifyAllResult, StoryVerifyStatus, ToolArgSpec, ToolEntry,
    TraceRecord, TraceScoreResult, TraceScoreSource, RESPONSIBILITIES,
};

pub type Result<T> = std::result::Result<T, HarnessInfraError>;

#[derive(Debug, Error)]
pub enum HarnessInfraError {
    #[error("database not found at {0}. Run: harness init")]
    MissingDatabase(String),
    #[error("schema file missing: {0}")]
    MissingSchema(String),
    #[error("brownfield import: missing {0}")]
    MissingBrownfieldPath(String),
    #[error("decision {0} has no verify_command. Configure one with: harness-cli decision add --id {0} --title <title> --verify \"<command>\"")]
    MissingDecisionVerifyCommand(String),
    #[error("story {0} has no verify_command. Configure one with: harness-cli story update --id {0} --verify \"<command>\"")]
    MissingStoryVerifyCommand(String),
    #[error("story update: story '{0}' not found")]
    StoryNotFound(String),
    #[error("tool register: tool '{0}' already exists with command '{1}'")]
    ToolAlreadyExists(String, String),
    #[error("tool remove: tool '{0}' not found")]
    ToolNotFound(String),
    #[error("tool register: command '{0}' was not found. Re-run with --force to register anyway.")]
    ToolCommandNotFound(String),
    #[error("{0}")]
    ToolValidation(#[from] crate::domain::ToolValidationError),
    #[error("backlog close: backlog item '{0}' not found")]
    BacklogNotFound(i64),
    #[error("trace '{0}' not found")]
    TraceNotFound(i64),
    #[error("no traces found")]
    NoTraces,
    #[error("story update: nothing to update")]
    EmptyStoryUpdate,
    #[error("trace --outcome {0} requires --next-action \"<what to do next>\" (unfinished work must record a resume hint)")]
    NextActionRequired(String),
    #[error("{0}")]
    EvidenceValidation(#[from] crate::domain::evidence::EvidenceValidationError),
    #[error("evidence add: artifact not found or unreadable at {0}")]
    EvidenceArtifactMissing(String),
    #[error("evidence add: anchor not found ({0})")]
    EvidenceAnchorNotFound(String),
    #[error("done-check requires --story <id> or --intake <id>")]
    DoneCheckTargetMissing,
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Outcome of one `tool check` scan. The CLI reports these facts; the agent
/// applies policy (skip / degrade / use) based on `status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCheckResult {
    pub name: String,
    pub kind: String,
    pub capability: Option<String>,
    pub status: String,
    pub detail: String,
}

pub trait HarnessRepository {
    fn init(&self) -> Result<InitResult>;
    fn migrate(&self) -> Result<MigrateResult>;
    fn import_brownfield(&self) -> Result<BrownfieldImportResult>;
    fn record_intake(&self, input: IntakeInput) -> Result<i64>;
    fn add_story(&self, input: StoryAddInput) -> Result<()>;
    fn update_story(&self, input: StoryUpdateInput) -> Result<()>;
    fn verify_story(&self, id: &str, capture: bool) -> Result<StoryVerifyResult>;
    fn verify_all_stories(&self) -> Result<StoryVerifyAllResult>;
    fn add_decision(&self, input: DecisionAddInput) -> Result<()>;
    fn verify_decision(&self, id: &str) -> Result<DecisionVerifyResult>;
    fn add_backlog(&self, input: BacklogAddInput) -> Result<i64>;
    fn close_backlog(&self, input: BacklogCloseInput) -> Result<()>;
    fn register_tool(&self, input: ToolRegisterInput) -> Result<()>;
    fn remove_tool(&self, name: &str) -> Result<()>;
    fn check_tools(&self, name: Option<String>) -> Result<Vec<ToolCheckResult>>;
    fn add_intervention(&self, input: InterventionAddInput) -> Result<i64>;
    fn record_trace(&self, input: TraceInput) -> Result<i64>;
    fn add_evidence(&self, input: EvidenceAddInput) -> Result<EvidenceAddResult>;
    fn list_evidence(&self, filter: EvidenceFilter) -> Result<Vec<EvidenceRecord>>;
    fn score_trace(&self, id: Option<i64>) -> Result<TraceScoreResult>;
    fn score_context(&self, id: i64) -> Result<ContextScoreResult>;
    fn story_verify_status(&self, id: &str) -> Result<StoryVerifyStatus>;
    fn query_matrix(&self) -> Result<Vec<StoryMatrixRecord>>;
    fn query_backlog(&self, filter: BacklogFilter) -> Result<Vec<BacklogRecord>>;
    fn query_decisions(&self) -> Result<Vec<DecisionRecord>>;
    fn query_intakes(&self) -> Result<Vec<IntakeRecord>>;
    fn query_traces(&self) -> Result<Vec<TraceRecord>>;
    fn query_friction(&self) -> Result<Vec<FrictionRecord>>;
    fn query_tools(
        &self,
        responsibility: Option<String>,
        capability: Option<String>,
    ) -> Result<Vec<ToolEntry>>;
    fn query_interventions(&self, filter: InterventionFilter) -> Result<Vec<InterventionRecord>>;
    fn query_stats(&self) -> Result<HarnessStats>;
    fn query_status(&self, filter: StatusFilter) -> Result<StatusReport>;
    fn query_recap(&self, filter: RecapFilter) -> Result<RecapReport>;
    fn done_check(&self, story_id: Option<String>, intake_id: Option<i64>)
        -> Result<DoneCheckReport>;
    fn audit(&self) -> Result<AuditResult>;
    fn propose(&self, commit: bool) -> Result<Vec<ImprovementProposal>>;
    fn query_sql(&self, sql: &str) -> Result<QueryTable>;
}

/// One story row read by `done_check`, named to keep the query closure tidy.
struct DoneCheckStoryRow {
    lane: String,
    status: String,
    verify_command: Option<String>,
    last_verified: Option<String>,
    unit: i64,
    integration: i64,
    e2e: i64,
    platform: i64,
    next_action: Option<String>,
    contract_doc: Option<String>,
}

#[derive(Debug)]
pub struct SqliteHarnessRepository {
    repo_root: PathBuf,
    db_path: PathBuf,
    schema_dir: PathBuf,
}

impl SqliteHarnessRepository {
    pub fn new(repo_root: PathBuf, db_path: PathBuf, schema_dir: PathBuf) -> Self {
        Self {
            repo_root,
            db_path,
            schema_dir,
        }
    }

    fn open_existing(&self) -> Result<Connection> {
        if !self.db_path.exists() {
            return Err(HarnessInfraError::MissingDatabase(
                self.db_path.display().to_string(),
            ));
        }

        let connection = Connection::open(&self.db_path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }

    fn open_or_create(&self) -> Result<Connection> {
        let connection = Connection::open(&self.db_path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }

    fn schema_version(connection: &Connection) -> Result<i64> {
        let version = connection
            .query_row(
                "SELECT COALESCE(MAX(version),0) FROM schema_version;",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);
        Ok(version)
    }

    fn apply_schema_v1(&self, connection: &Connection) -> Result<()> {
        let schema_path = self.schema_dir.join("001-init.sql");
        if !schema_path.exists() {
            return Err(HarnessInfraError::MissingSchema(
                schema_path.display().to_string(),
            ));
        }

        let schema = fs::read_to_string(schema_path)?;
        connection.execute_batch(&schema)?;
        Ok(())
    }

    fn apply_pending_migrations(
        &self,
        connection: &Connection,
        current_version: i64,
    ) -> Result<Vec<i64>> {
        let mut applied = Vec::new();
        for (version, path) in self.migration_files()? {
            if version > current_version {
                let sql = fs::read_to_string(path)?;
                connection.execute_batch(&sql)?;
                applied.push(version);
            }
        }
        Ok(applied)
    }

    fn migration_files(&self) -> Result<Vec<(i64, PathBuf)>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.schema_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("sql") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(prefix) = file_name.split('-').next() else {
                continue;
            };
            let Ok(version) = prefix.trim_start_matches('0').parse::<i64>() else {
                continue;
            };
            files.push((version, path));
        }
        files.sort_by_key(|(version, _)| *version);
        Ok(files)
    }

    fn import_matrix(&self, connection: &Connection) -> Result<usize> {
        let matrix_path = self.repo_root.join("_harness/docs/TEST_MATRIX.md");
        if !matrix_path.exists() {
            return Err(HarnessInfraError::MissingBrownfieldPath(
                matrix_path.display().to_string(),
            ));
        }

        let content = fs::read_to_string(matrix_path)?;
        let mut story_count = 0;
        let mut columns: Option<MatrixColumns> = None;

        for line in content.lines() {
            if !line.trim_start().starts_with('|') {
                continue;
            }

            let fields = markdown_table_fields(line);
            if fields.len() < 2 {
                continue;
            }

            if columns.is_none() {
                let candidate = MatrixColumns::from_header(&fields);
                if candidate.story.is_some() && candidate.status.is_some() {
                    columns = Some(candidate);
                }
                continue;
            }

            let columns = columns.as_ref().expect("matrix columns discovered");
            let id = field_at(&fields, columns.story).unwrap_or_default();
            let token = normalize_token(&id);
            if matches!(
                token.as_str(),
                "" | "story" | "tbd" | "todo" | "example" | "examples"
            ) || id.chars().all(|character| character == '-')
            {
                continue;
            }

            let mut title = field_at(&fields, columns.contract).unwrap_or_else(|| id.clone());
            if title.is_empty() {
                title = id.clone();
            }

            let status =
                normalize_story_status(&field_at(&fields, columns.status).unwrap_or_default());
            let unit = proof_from_cell(&field_at(&fields, columns.unit).unwrap_or_default());
            let integration =
                proof_from_cell(&field_at(&fields, columns.integration).unwrap_or_default());
            let e2e = proof_from_cell(&field_at(&fields, columns.e2e).unwrap_or_default());
            let platform =
                proof_from_cell(&field_at(&fields, columns.platform).unwrap_or_default());
            let evidence = columns
                .evidence
                .and_then(|index| evidence_from_fields(&fields, index));

            connection.execute(
                "INSERT INTO story (
                    id, title, risk_lane, contract_doc, status,
                    unit_proof, integration_proof, e2e_proof, platform_proof,
                    evidence, notes
                 ) VALUES (?1, ?2, 'high_risk', ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                    'Imported from _harness/docs/TEST_MATRIX.md by harness import brownfield.'
                 )
                 ON CONFLICT(id) DO UPDATE SET
                    title=excluded.title,
                    contract_doc=excluded.contract_doc,
                    status=excluded.status,
                    unit_proof=excluded.unit_proof,
                    integration_proof=excluded.integration_proof,
                    e2e_proof=excluded.e2e_proof,
                    platform_proof=excluded.platform_proof,
                    evidence=excluded.evidence,
                    notes=excluded.notes;",
                params![
                    id,
                    title,
                    field_at(&fields, columns.contract),
                    status,
                    unit,
                    integration,
                    e2e,
                    platform,
                    evidence,
                ],
            )?;
            story_count += 1;
        }

        Ok(story_count)
    }

    fn import_decisions(&self, connection: &Connection) -> Result<usize> {
        let decisions_dir = self.repo_root.join("docs/decisions");
        if !decisions_dir.is_dir() {
            return Err(HarnessInfraError::MissingBrownfieldPath(
                decisions_dir.display().to_string(),
            ));
        }

        let mut files = Vec::new();
        for entry in fs::read_dir(&decisions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("md") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if is_decision_file_name(file_name) {
                files.push(path);
            }
        }
        files.sort();

        let mut decision_count = 0;
        for path in files {
            let content = fs::read_to_string(&path)?;
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned();
            let title = content
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("# "))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&stem)
                .to_owned();
            let status =
                normalize_decision_status(&markdown_section_first_value(&content, "Status"));
            let doc_path = format!(
                "docs/decisions/{}",
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
            );

            connection.execute(
                "INSERT INTO decision (id, title, status, doc_path, notes)
                 VALUES (?1, ?2, ?3, ?4,
                    'Imported from docs/decisions by harness import brownfield.'
                 )
                 ON CONFLICT(id) DO UPDATE SET
                    title=excluded.title,
                    status=excluded.status,
                    doc_path=excluded.doc_path,
                    notes=excluded.notes;",
                params![stem, title, status, doc_path],
            )?;
            decision_count += 1;
        }

        Ok(decision_count)
    }

    fn import_backlog(&self, connection: &Connection) -> Result<usize> {
        let backlog_path = self.repo_root.join("_harness/docs/HARNESS_BACKLOG.md");
        if !backlog_path.exists() {
            return Ok(0);
        }

        let content = fs::read_to_string(backlog_path)?;
        let items = backlog_items(&content);
        let mut imported = 0;
        for item in items {
            if item.title.is_empty() || item.title == "Short name." {
                continue;
            }

            let risk = if item.risk.is_empty() {
                None
            } else {
                RiskLane::from_str(&item.risk)
                    .ok()
                    .map(|value| value.as_db_value().to_owned())
            };
            let status = normalize_backlog_status(&item.status);
            let discovered = empty_to_none(item.discovered_while);
            let pain = empty_to_none(item.current_pain);
            let suggestion = empty_to_none(item.suggested_improvement);

            connection.execute(
                "INSERT INTO backlog (
                    title, discovered_while, current_pain, suggested_improvement,
                    risk, status, notes
                 )
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6,
                    'Imported from _harness/docs/HARNESS_BACKLOG.md by harness import brownfield.'
                 WHERE NOT EXISTS (
                    SELECT 1 FROM backlog WHERE title=?1
                 );",
                params![item.title, discovered, pain, suggestion, risk, status],
            )?;
            imported += 1;
        }

        Ok(imported)
    }

    /// Copy an evidence artifact into the gitignored store and return its
    /// repo-relative pointer. Naming is content-addressed
    /// (`<kind>-<sha16><ext>`) so identical bytes resolve to the same file.
    fn store_evidence_artifact(
        &self,
        story_id: Option<&str>,
        trace_id: Option<i64>,
        kind: &str,
        sha256: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<String> {
        let key = match (story_id, trace_id) {
            (Some(story), _) => sanitize_key(story),
            (None, Some(trace)) => format!("trace-{trace}"),
            (None, None) => "unlinked".to_owned(),
        };
        let ext = file_name
            .rsplit_once('.')
            .map(|(_, ext)| format!(".{}", sanitize_key(ext)))
            .filter(|ext| ext.len() > 1)
            .unwrap_or_else(|| {
                if evidence::is_text_kind(kind) {
                    ".txt".to_owned()
                } else {
                    ".bin".to_owned()
                }
            });
        let short = &sha256[..16.min(sha256.len())];
        let relative = format!("_harness/evidence/{key}/{kind}-{short}{ext}");
        let absolute = self.repo_root.join(&relative);
        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&absolute, bytes)?;
        Ok(relative)
    }

    /// Check the four high-risk packet anchors live next to the story's
    /// contract doc (overview/execplan/design/validation).
    fn high_risk_anchors_present(&self, contract_doc: Option<&str>) -> (bool, String) {
        let Some(contract) = contract_doc.filter(|value| !value.trim().is_empty()) else {
            return (false, "no contract_doc to locate the packet folder".to_owned());
        };
        let folder = match Path::new(contract).parent() {
            Some(parent) => self.repo_root.join(parent),
            None => return (false, "contract_doc has no parent folder".to_owned()),
        };
        let anchors = ["overview.md", "execplan.md", "design.md", "validation.md"];
        let missing: Vec<&str> = anchors
            .iter()
            .copied()
            .filter(|anchor| !folder.join(anchor).exists())
            .collect();
        if missing.is_empty() {
            (true, "overview/execplan/design/validation present".to_owned())
        } else {
            (false, format!("missing: {}", missing.join(", ")))
        }
    }

    /// Capture a `story verify` log as a `kind='log'` evidence row, keeping the
    /// store small: keep-last per `(story, 'log', result)` and content dedup by
    /// sha256. A `fail→pass` transition naturally keeps both rows because
    /// keep-last is keyed by result (the fix evidence survives). See decision
    /// 0002 / US-004.
    fn capture_verify_log(
        &self,
        connection: &Connection,
        story_id: &str,
        command: &str,
        result: &str,
        log_text: &str,
    ) -> Result<i64> {
        let bytes = log_text.as_bytes();
        let sha256 = evidence::sha256_hex(bytes);

        // Content dedup: an identical log with the SAME result already captured
        // ⇒ refresh recency only. Scoping by result is essential: a fail→pass
        // transition that emits byte-identical output must NOT dedup against the
        // stale fail row, or no result='pass' row would ever be created.
        let duplicate: Option<i64> = connection
            .query_row(
                "SELECT id FROM evidence
                 WHERE story_id = ?1 AND kind = 'log' AND sha256 = ?2 AND result = ?3
                 ORDER BY id DESC LIMIT 1;",
                params![story_id, sha256, result],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if let Some(id) = duplicate {
            connection.execute(
                "UPDATE evidence SET created_at = datetime('now'), command = ?2 WHERE id = ?1;",
                params![id, command],
            )?;
            return Ok(id);
        }

        // Keep-last: drop the prior row + file for the SAME result.
        let mut statement = connection.prepare(
            "SELECT path FROM evidence WHERE story_id = ?1 AND kind = 'log' AND result = ?2;",
        )?;
        let stale_rows = statement.query_map(params![story_id, result], |row| {
            row.get::<_, String>(0)
        })?;
        for path in collect_rows(stale_rows)? {
            let _ = fs::remove_file(self.repo_root.join(&path));
        }
        drop(statement);
        connection.execute(
            "DELETE FROM evidence WHERE story_id = ?1 AND kind = 'log' AND result = ?2;",
            params![story_id, result],
        )?;

        let digest = evidence::build_digest("log", "verify.log", bytes);
        let stored_path = self.store_evidence_artifact(
            Some(story_id),
            None,
            "log",
            &sha256,
            "verify.log",
            bytes,
        )?;
        connection.execute(
            "INSERT INTO evidence
                (story_id, kind, path, sha256, bytes, digest, command, result, source)
             VALUES (?1, 'log', ?2, ?3, ?4, ?5, ?6, ?7, 'agent');",
            params![
                story_id,
                stored_path,
                sha256,
                bytes.len() as i64,
                digest,
                command,
                result,
            ],
        )?;
        Ok(connection.last_insert_rowid())
    }
}

impl HarnessRepository for SqliteHarnessRepository {
    fn init(&self) -> Result<InitResult> {
        if self.db_path.exists() {
            let connection = self.open_existing()?;
            let current = Self::schema_version(&connection).unwrap_or(0);
            if current == 0 {
                self.apply_schema_v1(&connection)?;
                self.apply_pending_migrations(&connection, 1)?;
                return Ok(InitResult::MigratedExisting {
                    db_path: self.db_path.clone(),
                });
            }

            return Ok(InitResult::Existing {
                db_path: self.db_path.clone(),
                version: current,
            });
        }

        let connection = self.open_or_create()?;
        self.apply_schema_v1(&connection)?;
        self.apply_pending_migrations(&connection, 1)?;
        Ok(InitResult::Created {
            db_path: self.db_path.clone(),
        })
    }

    fn migrate(&self) -> Result<MigrateResult> {
        let connection = self.open_existing()?;
        let current_version = Self::schema_version(&connection).unwrap_or(0);
        let applied = self.apply_pending_migrations(&connection, current_version)?;

        Ok(MigrateResult {
            current_version,
            applied,
        })
    }

    fn import_brownfield(&self) -> Result<BrownfieldImportResult> {
        let connection = self.open_existing()?;
        let stories = self.import_matrix(&connection)?;
        let decisions = self.import_decisions(&connection)?;
        let backlog_items = self.import_backlog(&connection)?;

        Ok(BrownfieldImportResult {
            stories,
            decisions,
            backlog_items,
        })
    }

    fn record_intake(&self, input: IntakeInput) -> Result<i64> {
        let connection = self.open_existing()?;
        connection.execute(
            "INSERT INTO intake (
                input_type, summary, risk_lane, risk_flags, affected_docs, story_id, notes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            params![
                input.input_type.as_db_value(),
                input.summary,
                input.risk_lane.as_db_value(),
                input.risk_flags.as_json_text(),
                input.affected_docs.as_json_text(),
                input.story_id,
                input.notes,
            ],
        )?;

        Ok(connection.last_insert_rowid())
    }

    fn add_story(&self, input: StoryAddInput) -> Result<()> {
        let connection = self.open_existing()?;
        connection.execute(
            "INSERT INTO story (id, title, risk_lane, contract_doc, verify_command, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![
                input.id,
                input.title,
                input.risk_lane.as_db_value(),
                input.contract_doc,
                input.verify_command,
                input.notes,
            ],
        )?;
        Ok(())
    }

    fn update_story(&self, input: StoryUpdateInput) -> Result<()> {
        if input.status.is_none()
            && input.evidence.is_none()
            && input.unit.is_none()
            && input.integration.is_none()
            && input.e2e.is_none()
            && input.platform.is_none()
            && input.verify_command.is_none()
            && input.next_action.is_none()
        {
            return Err(HarnessInfraError::EmptyStoryUpdate);
        }

        let connection = self.open_existing()?;
        connection.execute(
            "UPDATE story SET
                status=COALESCE(?1, status),
                evidence=COALESCE(?2, evidence),
                unit_proof=COALESCE(?3, unit_proof),
                integration_proof=COALESCE(?4, integration_proof),
                e2e_proof=COALESCE(?5, e2e_proof),
                platform_proof=COALESCE(?6, platform_proof),
                verify_command=COALESCE(?7, verify_command),
                next_action=COALESCE(?9, next_action),
                next_action_at=CASE WHEN ?9 IS NOT NULL THEN datetime('now') ELSE next_action_at END
             WHERE id=?8;",
            params![
                input.status,
                input.evidence,
                input.unit.map(|value| value.0),
                input.integration.map(|value| value.0),
                input.e2e.map(|value| value.0),
                input.platform.map(|value| value.0),
                input.verify_command,
                input.id,
                input.next_action,
            ],
        )?;

        if connection.changes() == 0 {
            return Err(HarnessInfraError::StoryNotFound(input.id));
        }
        Ok(())
    }

    fn verify_story(&self, id: &str, capture: bool) -> Result<StoryVerifyResult> {
        let connection = self.open_existing()?;
        let verify_command = connection
            .query_row(
                "SELECT verify_command FROM story WHERE id=?1;",
                params![id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| HarnessInfraError::MissingStoryVerifyCommand(id.to_owned()))?;

        let (shell, flag) = verifier_shell();
        let output = Command::new(shell)
            .arg(flag)
            .arg(&verify_command)
            .current_dir(&self.repo_root)
            .output()?;
        let result = if output.status.success() {
            "pass"
        } else {
            "fail"
        }
        .to_owned();
        connection.execute(
            "UPDATE story
             SET last_verified_at=datetime('now'), last_verified_result=?1
             WHERE id=?2;",
            params![result, id],
        )?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        // Auto-capture (default-on): a proof boolean must always have a fresh
        // log backing it. Dedup keeps the store small; see decision 0002.
        let evidence_id = if capture {
            let log_text = format!("$ {verify_command}\n{stdout}{stderr}");
            Some(self.capture_verify_log(&connection, id, &verify_command, &result, &log_text)?)
        } else {
            None
        };

        Ok(StoryVerifyResult {
            command: verify_command,
            stdout,
            stderr,
            result,
            evidence_id,
        })
    }

    fn verify_all_stories(&self) -> Result<StoryVerifyAllResult> {
        let connection = self.open_existing()?;
        let mut statement =
            connection.prepare("SELECT id, title, verify_command FROM story ORDER BY id;")?;
        let story_rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let stories = collect_rows(story_rows)?;
        let mut items = Vec::new();

        for (id, title, verify_command) in stories {
            let Some(command) = verify_command.filter(|value| !value.trim().is_empty()) else {
                items.push(StoryVerifyAllItem {
                    id,
                    title,
                    command: None,
                    result: "skipped".to_owned(),
                    stdout: String::new(),
                    stderr: String::new(),
                });
                continue;
            };

            let (shell, flag) = verifier_shell();
            let output = Command::new(shell)
                .arg(flag)
                .arg(&command)
                .current_dir(&self.repo_root)
                .output()?;
            let result = if output.status.success() {
                "pass"
            } else {
                "fail"
            }
            .to_owned();
            connection.execute(
                "UPDATE story
                 SET last_verified_at=datetime('now'), last_verified_result=?1
                 WHERE id=?2;",
                params![result, id],
            )?;
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            // Auto-capture like single `story verify`, so a story verified only
            // in batch still gets the evidence pass-log that done-check requires.
            let log_text = format!("$ {command}\n{stdout}{stderr}");
            self.capture_verify_log(&connection, &id, &command, &result, &log_text)?;
            items.push(StoryVerifyAllItem {
                id,
                title,
                command: Some(command),
                result,
                stdout,
                stderr,
            });
        }

        Ok(StoryVerifyAllResult { items })
    }

    fn add_decision(&self, input: DecisionAddInput) -> Result<()> {
        let connection = self.open_existing()?;
        connection.execute(
            "INSERT INTO decision (id, title, status, doc_path, verify_command, predicted_impact, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            params![
                input.id,
                input.title,
                input.status,
                input.doc_path,
                input.verify_command,
                input.predicted_impact,
                input.notes,
            ],
        )?;
        Ok(())
    }

    fn verify_decision(&self, id: &str) -> Result<DecisionVerifyResult> {
        let connection = self.open_existing()?;
        let verify_command = connection
            .query_row(
                "SELECT verify_command FROM decision WHERE id=?1;",
                params![id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| HarnessInfraError::MissingDecisionVerifyCommand(id.to_owned()))?;

        let (shell, flag) = verifier_shell();
        let status = Command::new(shell)
            .arg(flag)
            .arg(&verify_command)
            .current_dir(&self.repo_root)
            .status()?;
        let result = if status.success() { "pass" } else { "fail" }.to_owned();
        connection.execute(
            "UPDATE decision
             SET last_verified_at=datetime('now'), last_verified_result=?1
             WHERE id=?2;",
            params![result, id],
        )?;

        Ok(DecisionVerifyResult {
            command: verify_command,
            result,
        })
    }

    fn add_backlog(&self, input: BacklogAddInput) -> Result<i64> {
        let connection = self.open_existing()?;
        connection.execute(
            "INSERT INTO backlog (
                title, discovered_while, current_pain, suggested_improvement,
                risk, predicted_impact, notes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            params![
                input.title,
                input.discovered_while,
                input.current_pain,
                input.suggestion,
                input.risk.map(|value| value.as_db_value().to_owned()),
                input.predicted_impact,
                input.notes,
            ],
        )?;
        Ok(connection.last_insert_rowid())
    }

    fn close_backlog(&self, input: BacklogCloseInput) -> Result<()> {
        let connection = self.open_existing()?;
        connection.execute(
            "UPDATE backlog
             SET status=?1, actual_outcome=?2, implemented_at=datetime('now')
             WHERE id=?3;",
            params![input.status, input.actual_outcome, input.id],
        )?;

        if connection.changes() == 0 {
            return Err(HarnessInfraError::BacklogNotFound(input.id));
        }
        Ok(())
    }

    fn register_tool(&self, input: ToolRegisterInput) -> Result<()> {
        validate_tool_description(&input.description)?;
        // Only exec-probed kinds are PATH-checked at register time. mcp/skill/http
        // are not on PATH by nature, so registering intent always succeeds; their
        // presence is resolved later by `tool check` via scan_target.
        let exec_probed = matches!(input.kind.as_str(), "cli" | "binary");
        if exec_probed && !input.force && !command_available(&self.repo_root, &input.command) {
            return Err(HarnessInfraError::ToolCommandNotFound(input.command));
        }

        let connection = self.open_existing()?;
        let existing = connection
            .query_row(
                "SELECT command FROM tool WHERE name=?1;",
                params![input.name],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(command) = existing {
            return Err(HarnessInfraError::ToolAlreadyExists(input.name, command));
        }

        connection.execute(
            "INSERT INTO tool
                (name, provider, command, description, args, responsibility, since,
                 kind, capability, scan_target, status)
             VALUES (?1, 'custom', ?2, ?3, ?4, ?5, 'registered', ?6, ?7, ?8, 'unknown');",
            params![
                input.name,
                input.command,
                input.description,
                tool_args_json(&input.args),
                input.responsibility,
                input.kind,
                input.capability,
                input.scan_target,
            ],
        )?;
        Ok(())
    }

    fn remove_tool(&self, name: &str) -> Result<()> {
        let connection = self.open_existing()?;
        connection.execute("DELETE FROM tool WHERE name=?1;", params![name])?;
        if connection.changes() == 0 {
            return Err(HarnessInfraError::ToolNotFound(name.to_owned()));
        }
        Ok(())
    }

    fn check_tools(&self, name: Option<String>) -> Result<Vec<ToolCheckResult>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT name, kind, command, scan_target, capability FROM tool
             WHERE (?1 IS NULL OR name = ?1)
             ORDER BY name;",
        )?;
        let rows = statement.query_map(params![name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        let tools = collect_rows(rows)?;

        let mut results = Vec::with_capacity(tools.len());
        for (name, kind, command, scan_target, capability) in tools {
            let (status, detail) =
                scan_tool_status(&self.repo_root, &kind, &command, scan_target.as_deref());
            connection.execute(
                "UPDATE tool SET status=?1, checked_at=datetime('now') WHERE name=?2;",
                params![status, name],
            )?;
            results.push(ToolCheckResult {
                name,
                kind,
                capability,
                status: status.to_owned(),
                detail,
            });
        }
        Ok(results)
    }

    fn add_intervention(&self, input: InterventionAddInput) -> Result<i64> {
        let connection = self.open_existing()?;
        connection.execute(
            "INSERT INTO intervention (trace_id, story_id, type, description, source, impact)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![
                input.trace_id,
                input.story_id,
                input.intervention_type,
                input.description,
                input.source,
                input.impact,
            ],
        )?;
        Ok(connection.last_insert_rowid())
    }

    fn record_trace(&self, input: TraceInput) -> Result<i64> {
        // Resume continuity: unfinished outcomes MUST carry a next-action hint
        // (anti-TODO-graveyard). A completed outcome clears the live pointer.
        let next_action = input
            .next_action
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        let outcome = input.outcome.as_deref().unwrap_or_default();
        if matches!(outcome, "partial" | "blocked" | "failed") && next_action.is_none() {
            return Err(HarnessInfraError::NextActionRequired(outcome.to_owned()));
        }

        let connection = self.open_existing()?;
        connection.execute(
            "INSERT INTO trace (
                task_summary, intake_id, story_id, agent,
                actions_taken, files_read, files_changed, decisions_made, errors,
                outcome, duration_seconds, token_estimate, harness_friction, notes,
                next_action
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15);",
            params![
                input.task_summary,
                input.intake_id,
                input.story_id,
                input.agent,
                input.actions.as_json_text(),
                input.files_read.as_json_text(),
                input.files_changed.as_json_text(),
                input.decisions.as_json_text(),
                input.errors.as_json_text(),
                input.outcome,
                input.duration_seconds,
                input.token_estimate,
                input.friction,
                input.notes,
                next_action,
            ],
        )?;
        let trace_id = connection.last_insert_rowid();

        // Sync the live story pointer when the trace is linked to a story.
        if let Some(story_id) = input.story_id.as_deref() {
            if outcome == "completed" {
                connection.execute(
                    "UPDATE story SET next_action = NULL, next_action_at = NULL WHERE id = ?1;",
                    params![story_id],
                )?;
            } else if let Some(action) = next_action.as_deref() {
                connection.execute(
                    "UPDATE story
                     SET next_action = ?1, next_action_at = datetime('now')
                     WHERE id = ?2;",
                    params![action, story_id],
                )?;
            }
        }

        Ok(trace_id)
    }

    fn add_evidence(&self, input: EvidenceAddInput) -> Result<EvidenceAddResult> {
        let kind = evidence::validate_kind(&input.kind)?;
        let source = evidence::validate_source(&input.source)?;
        if input.story_id.is_none() && input.trace_id.is_none() {
            return Err(evidence::EvidenceValidationError::MissingAnchor.into());
        }

        // Resolve the artifact path (relative paths anchor at the repo root).
        let raw_path = Path::new(&input.path);
        let abs_source = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            self.repo_root.join(raw_path)
        };
        let bytes = fs::read(&abs_source)
            .map_err(|_| HarnessInfraError::EvidenceArtifactMissing(input.path.clone()))?;
        let sha256 = evidence::sha256_hex(&bytes);
        let byte_len = bytes.len() as i64;
        let file_name = abs_source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("artifact");
        let digest = evidence::build_digest(&kind, file_name, &bytes);

        let connection = self.open_existing()?;
        // Validate the anchor BEFORE copying the artifact: otherwise a bad
        // --story/--trace would leave an orphan file on disk and surface a raw
        // foreign-key error instead of a clear message.
        if let Some(story_id) = input.story_id.as_deref() {
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM story WHERE id = ?1);",
                params![story_id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(HarnessInfraError::EvidenceAnchorNotFound(format!(
                    "story {story_id}"
                )));
            }
        }
        if let Some(trace_id) = input.trace_id {
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM trace WHERE id = ?1);",
                params![trace_id],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(HarnessInfraError::EvidenceAnchorNotFound(format!(
                    "trace {trace_id}"
                )));
            }
        }
        // Content dedup: an identical artifact under the same anchor+kind is not
        // copied or re-inserted; we only refresh its recency.
        let existing: Option<(i64, String)> = connection
            .query_row(
                "SELECT id, path FROM evidence
                 WHERE sha256 = ?1 AND kind = ?2
                   AND (story_id IS ?3) AND (trace_id IS ?4)
                 ORDER BY id DESC LIMIT 1;",
                params![sha256, kind, input.story_id, input.trace_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((id, path)) = existing {
            connection.execute(
                "UPDATE evidence SET created_at = datetime('now') WHERE id = ?1;",
                params![id],
            )?;
            return Ok(EvidenceAddResult {
                id,
                path,
                sha256,
                bytes: byte_len,
                deduped: true,
            });
        }

        let stored_path = self.store_evidence_artifact(
            input.story_id.as_deref(),
            input.trace_id,
            &kind,
            &sha256,
            file_name,
            &bytes,
        )?;
        connection.execute(
            "INSERT INTO evidence
                (story_id, trace_id, kind, path, sha256, bytes, digest, command, result, source, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);",
            params![
                input.story_id,
                input.trace_id,
                kind,
                stored_path,
                sha256,
                byte_len,
                digest,
                input.command,
                input.result,
                source,
                input.notes,
            ],
        )?;
        Ok(EvidenceAddResult {
            id: connection.last_insert_rowid(),
            path: stored_path,
            sha256,
            bytes: byte_len,
            deduped: false,
        })
    }

    fn list_evidence(&self, filter: EvidenceFilter) -> Result<Vec<EvidenceRecord>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT id, created_at, story_id, trace_id, kind, path, sha256, bytes,
                    digest, command, result, source, notes
             FROM evidence
             WHERE (?1 IS NULL OR story_id = ?1)
               AND (?2 IS NULL OR trace_id = ?2)
               AND (?3 IS NULL OR kind = ?3)
             ORDER BY id DESC;",
        )?;
        let rows = statement.query_map(
            params![filter.story_id, filter.trace_id, filter.kind],
            |row| {
                Ok(EvidenceRecord {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    story_id: row.get(2)?,
                    trace_id: row.get(3)?,
                    kind: row.get(4)?,
                    path: row.get(5)?,
                    sha256: row.get(6)?,
                    bytes: row.get(7)?,
                    digest: row.get(8)?,
                    command: row.get(9)?,
                    result: row.get(10)?,
                    source: row.get(11)?,
                    notes: row.get(12)?,
                })
            },
        )?;
        collect_rows(rows)
    }

    fn score_trace(&self, id: Option<i64>) -> Result<TraceScoreResult> {
        let connection = self.open_existing()?;
        let sql = match id {
            Some(_) => {
                "SELECT
                    trace.id,
                    trace.task_summary,
                    trace.intake_id,
                    intake.risk_lane,
                    trace.agent,
                    trace.actions_taken,
                    trace.files_read,
                    trace.files_changed,
                    trace.decisions_made,
                    trace.errors,
                    trace.outcome,
                    trace.duration_seconds,
                    trace.token_estimate,
                    trace.harness_friction,
                    trace.notes
                 FROM trace
                 LEFT JOIN intake ON intake.id = trace.intake_id
                 WHERE trace.id = ?1"
            }
            None => {
                "SELECT
                    trace.id,
                    trace.task_summary,
                    trace.intake_id,
                    intake.risk_lane,
                    trace.agent,
                    trace.actions_taken,
                    trace.files_read,
                    trace.files_changed,
                    trace.decisions_made,
                    trace.errors,
                    trace.outcome,
                    trace.duration_seconds,
                    trace.token_estimate,
                    trace.harness_friction,
                    trace.notes
                 FROM trace
                 LEFT JOIN intake ON intake.id = trace.intake_id
                 ORDER BY trace.id DESC
                 LIMIT 1"
            }
        };

        let source = if let Some(id) = id {
            connection
                .query_row(sql, params![id], trace_score_source_from_row)
                .optional()?
                .ok_or(HarnessInfraError::TraceNotFound(id))?
        } else {
            connection
                .query_row(sql, [], trace_score_source_from_row)
                .optional()?
                .ok_or(HarnessInfraError::NoTraces)?
        };

        Ok(score_trace(source))
    }

    fn score_context(&self, id: i64) -> Result<ContextScoreResult> {
        let connection = self.open_existing()?;
        let source = connection
            .query_row(
                "SELECT
                    trace.id,
                    intake.risk_lane,
                    trace.story_id,
                    trace.files_read,
                    trace.files_changed,
                    trace.outcome
                 FROM trace
                 LEFT JOIN intake ON intake.id = trace.intake_id
                 WHERE trace.id=?1;",
                params![id],
                |row| {
                    Ok(ContextScoreSource {
                        id: row.get(0)?,
                        risk_lane: row.get(1)?,
                        story_id: row.get(2)?,
                        files_read: row.get(3)?,
                        files_changed: row.get(4)?,
                        outcome: row.get(5)?,
                    })
                },
            )
            .optional()?
            .ok_or(HarnessInfraError::TraceNotFound(id))?;

        Ok(score_context(source))
    }

    fn story_verify_status(&self, id: &str) -> Result<StoryVerifyStatus> {
        let connection = self.open_existing()?;
        connection
            .query_row(
                "SELECT id, verify_command, last_verified_result FROM story WHERE id=?1;",
                params![id],
                |row| {
                    Ok(StoryVerifyStatus {
                        id: row.get(0)?,
                        verify_command: row.get(1)?,
                        last_verified_result: row.get(2)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| HarnessInfraError::StoryNotFound(id.to_owned()))
    }

    fn query_matrix(&self) -> Result<Vec<StoryMatrixRecord>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT id, title, status, unit_proof, integration_proof, e2e_proof, platform_proof, evidence
             FROM story ORDER BY id;",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(StoryMatrixRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                unit: row.get(3)?,
                integration: row.get(4)?,
                e2e: row.get(5)?,
                platform: row.get(6)?,
                evidence: row.get(7)?,
            })
        })?;

        collect_rows(rows)
    }

    fn query_backlog(&self, filter: BacklogFilter) -> Result<Vec<BacklogRecord>> {
        let connection = self.open_existing()?;
        let where_clause = match filter {
            BacklogFilter::All => "",
            BacklogFilter::Open => "WHERE status IN ('proposed', 'accepted')",
            BacklogFilter::Closed => "WHERE status IN ('implemented', 'rejected')",
        };
        let sql = format!(
            "SELECT id, title, status, risk, predicted_impact, actual_outcome
             FROM backlog {where_clause} ORDER BY status, id;"
        );
        let mut statement = connection.prepare(&sql)?;

        let rows = statement.query_map([], |row| {
            Ok(BacklogRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                risk: row.get(3)?,
                predicted_impact: row.get(4)?,
                actual_outcome: row.get(5)?,
            })
        })?;

        collect_rows(rows)
    }

    fn query_decisions(&self) -> Result<Vec<DecisionRecord>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT id, title, status, last_verified_at, last_verified_result
             FROM decision ORDER BY id;",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                last_verified_at: row.get(3)?,
                last_verified_result: row.get(4)?,
            })
        })?;

        collect_rows(rows)
    }

    fn query_intakes(&self) -> Result<Vec<IntakeRecord>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT id, created_at, input_type, risk_lane, summary
             FROM intake ORDER BY id DESC LIMIT 20;",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(IntakeRecord {
                id: row.get(0)?,
                created_at: row.get(1)?,
                input_type: row.get(2)?,
                risk_lane: row.get(3)?,
                summary: row.get(4)?,
            })
        })?;

        collect_rows(rows)
    }

    fn query_traces(&self) -> Result<Vec<TraceRecord>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT id, created_at, outcome, task_summary, harness_friction
             FROM trace ORDER BY id DESC LIMIT 20;",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(TraceRecord {
                id: row.get(0)?,
                created_at: row.get(1)?,
                outcome: row.get(2)?,
                task_summary: row.get(3)?,
                harness_friction: row.get(4)?,
            })
        })?;

        collect_rows(rows)
    }

    fn query_friction(&self) -> Result<Vec<FrictionRecord>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT
                trace.id,
                trace.created_at,
                intake.risk_lane,
                intake.input_type,
                trace.task_summary,
                trace.harness_friction
             FROM trace
             LEFT JOIN intake ON intake.id = trace.intake_id
             WHERE trace.harness_friction IS NOT NULL
             ORDER BY trace.id DESC;",
        )?;

        let rows = statement.query_map([], |row| {
            Ok(FrictionRecord {
                id: row.get(0)?,
                created_at: row.get(1)?,
                risk_lane: row.get(2)?,
                input_type: row.get(3)?,
                task_summary: row.get(4)?,
                harness_friction: row.get(5)?,
            })
        })?;

        collect_rows(rows)
    }

    fn query_tools(
        &self,
        responsibility: Option<String>,
        capability: Option<String>,
    ) -> Result<Vec<ToolEntry>> {
        let connection = self.open_existing()?;
        let mut tools = compiled_tool_registry();
        let mut statement = connection.prepare(
            "SELECT provider, name, command, description, args, responsibility, since,
                    kind, capability, scan_target, status, checked_at
             FROM tool ORDER BY name;",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ToolEntry {
                provider: row.get(0)?,
                name: row.get(1)?,
                command: row.get(2)?,
                description: row.get(3)?,
                args: parse_stored_tool_args(row.get::<_, Option<String>>(4)?.as_deref()),
                responsibility: row.get(5)?,
                source: "registered".to_owned(),
                since: row.get(6)?,
                kind: row.get(7)?,
                capability: row.get(8)?,
                scan_target: row.get(9)?,
                status: row.get(10)?,
                checked_at: row.get(11)?,
            })
        })?;
        tools.extend(collect_rows(rows)?);
        if let Some(responsibility) = responsibility {
            let normalized = normalize_token(&responsibility);
            tools.retain(|tool| normalize_token(&tool.responsibility) == normalized);
        }
        if let Some(capability) = capability {
            let normalized = normalize_token(&capability);
            tools.retain(|tool| {
                tool.capability
                    .as_deref()
                    .is_some_and(|value| normalize_token(value) == normalized)
            });
        }
        Ok(tools)
    }

    fn query_interventions(&self, filter: InterventionFilter) -> Result<Vec<InterventionRecord>> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(
            "SELECT id, created_at, trace_id, story_id, type, description, source, impact
             FROM intervention
             WHERE (?1 IS NULL OR trace_id = ?1)
               AND (?2 IS NULL OR story_id = ?2)
               AND (?3 IS NULL OR type = ?3)
             ORDER BY id DESC;",
        )?;
        let rows = statement.query_map(
            params![filter.trace_id, filter.story_id, filter.intervention_type],
            |row| {
                Ok(InterventionRecord {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    trace_id: row.get(2)?,
                    story_id: row.get(3)?,
                    intervention_type: row.get(4)?,
                    description: row.get(5)?,
                    source: row.get(6)?,
                    impact: row.get(7)?,
                })
            },
        )?;
        collect_rows(rows)
    }

    fn query_stats(&self) -> Result<HarnessStats> {
        let connection = self.open_existing()?;
        connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM intake) AS intakes,
                    (SELECT COUNT(*) FROM story) AS stories,
                    (SELECT COUNT(*) FROM decision) AS decisions,
                    (SELECT COUNT(*) FROM backlog) AS backlog_items,
                    (SELECT COUNT(*) FROM trace) AS traces;",
                [],
                |row| {
                    Ok(HarnessStats {
                        intakes: row.get(0)?,
                        stories: row.get(1)?,
                        decisions: row.get(2)?,
                        backlog_items: row.get(3)?,
                        traces: row.get(4)?,
                    })
                },
            )
            .map_err(HarnessInfraError::from)
    }

    fn query_status(&self, filter: StatusFilter) -> Result<StatusReport> {
        let connection = self.open_existing()?;
        let lane = filter.lane.as_deref();
        let limit = filter.limit;

        // ĐANG LÀM — in-progress stories, oldest first.
        let mut active_stmt = connection.prepare(
            "SELECT id, title, risk_lane, next_action FROM story
             WHERE status = 'in_progress' AND (?1 IS NULL OR risk_lane = ?1)
             ORDER BY created_at, id;",
        )?;
        let active_rows = active_stmt.query_map(params![lane], |row| {
            Ok(StatusStory {
                id: row.get(0)?,
                title: row.get(1)?,
                lane: row.get(2)?,
                next_action: row.get(3)?,
            })
        })?;
        let active = StatusSection::capped(collect_rows(active_rows)?, limit);

        // CẦN PROOF — implemented but not verified-pass, or no proof flags set.
        let mut proof_stmt = connection.prepare(
            "SELECT id, title, risk_lane, last_verified_result,
                    unit_proof, integration_proof, e2e_proof, platform_proof
             FROM story
             WHERE status = 'implemented' AND (?1 IS NULL OR risk_lane = ?1)
               AND (
                 last_verified_result IS NULL
                 OR last_verified_result <> 'pass'
                 OR (unit_proof = 0 AND integration_proof = 0
                     AND e2e_proof = 0 AND platform_proof = 0)
               )
             ORDER BY id;",
        )?;
        let proof_rows = proof_stmt.query_map(params![lane], |row| {
            Ok(StatusProofGap {
                id: row.get(0)?,
                title: row.get(1)?,
                lane: row.get(2)?,
                verify_result: row.get(3)?,
                unit: row.get(4)?,
                integration: row.get(5)?,
                e2e: row.get(6)?,
                platform: row.get(7)?,
            })
        })?;
        let needs_proof = StatusSection::capped(collect_rows(proof_rows)?, limit);

        // RESUME — unresolved live story pointers plus unlinked unfinished traces.
        let mut resume_stmt = connection.prepare(
            "SELECT trace.id,
                    trace.story_id,
                    trace.outcome,
                    COALESCE(story.next_action, trace.next_action),
                    trace.task_summary
             FROM trace
             LEFT JOIN story ON story.id = trace.story_id
             WHERE trace.outcome IN ('partial','blocked','failed')
               AND (
                 (trace.story_id IS NULL AND ?1 IS NULL)
                 OR (
                   story.next_action IS NOT NULL
                   AND (?1 IS NULL OR story.risk_lane = ?1)
                   AND NOT EXISTS (
                     SELECT 1 FROM trace newer
                     WHERE newer.story_id = trace.story_id
                       AND newer.id > trace.id
                   )
                 )
               )
             ORDER BY trace.id DESC;",
        )?;
        let resume_rows = resume_stmt.query_map(params![lane], |row| {
            Ok(StatusResume {
                trace_id: row.get(0)?,
                story_id: row.get(1)?,
                outcome: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                next_action: row.get(3)?,
                task_summary: row.get(4)?,
            })
        })?;
        let resume = StatusSection::capped(collect_rows(resume_rows)?, limit);

        // BACKLOG MỞ — open items, high-risk first.
        let mut backlog_stmt = connection.prepare(
            "SELECT id, risk, title, predicted_impact FROM backlog
             WHERE status IN ('proposed','accepted')
               AND (?1 IS NULL OR risk = ?1)
             ORDER BY CASE risk WHEN 'high_risk' THEN 0 WHEN 'normal' THEN 1
                                WHEN 'tiny' THEN 2 ELSE 3 END, id;",
        )?;
        let backlog_rows = backlog_stmt.query_map(params![lane], |row| {
            Ok(StatusBacklogItem {
                id: row.get(0)?,
                risk: row.get(1)?,
                title: row.get(2)?,
                predicted: row.get(3)?,
            })
        })?;
        let backlog = StatusSection::capped(collect_rows(backlog_rows)?, limit);

        // INTERVENTION — most recent.
        let mut intervention_stmt = connection.prepare(
            "SELECT id, type, source, trace_id, story_id FROM intervention
             ORDER BY id DESC;",
        )?;
        let intervention_rows = intervention_stmt.query_map([], |row| {
            Ok(StatusInterventionItem {
                id: row.get(0)?,
                intervention_type: row.get(1)?,
                source: row.get(2)?,
                trace_id: row.get(3)?,
                story_id: row.get(4)?,
            })
        })?;
        let interventions = StatusSection::capped(collect_rows(intervention_rows)?, limit);

        // HOẠT ĐỘNG GẦN — most recent traces.
        let mut recent_stmt = connection.prepare(
            "SELECT id, outcome, task_summary FROM trace ORDER BY id DESC;",
        )?;
        let recent_rows = recent_stmt.query_map([], |row| {
            Ok(StatusActivity {
                trace_id: row.get(0)?,
                outcome: row.get(1)?,
                task_summary: row.get(2)?,
            })
        })?;
        let recent = StatusSection::capped(collect_rows(recent_rows)?, limit);

        // Drift header — reuse the audit entropy score (one line, no full audit
        // print). drift_groups = number of audit categories with findings.
        let audit = self.audit()?;
        let drift_groups = [
            audit.orphaned_stories.len(),
            audit.unverified_stories.len(),
            audit.unverified_decisions.len(),
            audit.backlog_without_outcomes.len(),
            audit.stale_stories.len(),
            audit.broken_tools.len(),
        ]
        .iter()
        .filter(|count| **count > 0)
        .count() as i64;

        Ok(StatusReport {
            entropy_score: audit.entropy_score(),
            drift_groups,
            active,
            needs_proof,
            resume,
            backlog,
            interventions,
            recent,
        })
    }

    fn query_recap(&self, filter: RecapFilter) -> Result<RecapReport> {
        let connection = self.open_existing()?;

        // Build the trace selection from the provided filters (all ANDed).
        let mut clauses: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();
        if let Some(story) = filter.story_id.as_deref() {
            clauses.push(format!("story_id = ?{}", binds.len() + 1));
            binds.push(story.to_owned());
        }
        if let Some(prefix) = filter.epic_prefix.as_deref() {
            clauses.push(format!("story_id LIKE ?{}", binds.len() + 1));
            binds.push(format!("{}%", prefix.replace('%', "\\%")));
        }
        if let Some(since) = filter.since.as_deref() {
            clauses.push(format!("created_at >= ?{}", binds.len() + 1));
            binds.push(since.to_owned());
        }
        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT id, created_at, outcome, files_changed, decisions_made, harness_friction
             FROM trace {where_clause} ORDER BY id;"
        );
        let mut statement = connection.prepare(&sql)?;
        let bind_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|value| value as &dyn rusqlite::ToSql).collect();
        let rows = statement.query_map(bind_refs.as_slice(), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let traces = collect_rows(rows)?;

        let mut report = RecapReport {
            scope: recap_scope(&filter),
            first_at: None,
            last_at: None,
            trace_count: traces.len() as i64,
            completed: 0,
            partial: 0,
            blocked: 0,
            failed: 0,
            files: Vec::new(),
            friction: Vec::new(),
            decisions: Vec::new(),
            interventions: Vec::new(),
        };

        let mut file_counts: BTreeMap<String, i64> = BTreeMap::new();
        let mut friction_counts: BTreeMap<String, i64> = BTreeMap::new();
        let mut decisions: BTreeSet<String> = BTreeSet::new();
        let mut trace_ids: BTreeSet<i64> = BTreeSet::new();

        for (id, created_at, outcome, files_changed, decisions_made, friction) in &traces {
            trace_ids.insert(*id);
            if report.first_at.is_none() {
                report.first_at = Some(created_at.clone());
            }
            report.last_at = Some(created_at.clone());

            match outcome.as_deref() {
                Some("completed") => report.completed += 1,
                Some("partial") => report.partial += 1,
                Some("blocked") => report.blocked += 1,
                Some("failed") => report.failed += 1,
                _ => {}
            }
            for file in jsonish_list(files_changed.as_deref()) {
                *file_counts.entry(file).or_insert(0) += 1;
            }
            for decision in jsonish_list(decisions_made.as_deref()) {
                if decision != "none" {
                    decisions.insert(decision);
                }
            }
            if let Some(text) = friction.as_deref() {
                let trimmed = text.trim();
                if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("none") {
                    let component = attribute_friction_component(trimmed);
                    *friction_counts.entry(component).or_insert(0) += 1;
                }
            }
        }

        report.files = top_counts(file_counts, 10);
        report.friction = top_counts(friction_counts, RESPONSIBILITIES.len());
        report.decisions = decisions.into_iter().collect();

        // Interventions attached to the matching traces, grouped by type.
        let mut intervention_counts: BTreeMap<String, i64> = BTreeMap::new();
        let mut intervention_stmt =
            connection.prepare("SELECT type, trace_id FROM intervention;")?;
        let intervention_rows = intervention_stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        })?;
        for (kind, trace_id) in collect_rows(intervention_rows)? {
            if let Some(trace_id) = trace_id {
                if trace_ids.contains(&trace_id) {
                    *intervention_counts.entry(kind).or_insert(0) += 1;
                }
            }
        }
        report.interventions = top_counts(intervention_counts, usize::MAX);

        Ok(report)
    }

    fn done_check(
        &self,
        story_id: Option<String>,
        intake_id: Option<i64>,
    ) -> Result<DoneCheckReport> {
        let connection = self.open_existing()?;

        // Resolve the target story. --story wins; --intake resolves via its
        // linked story_id (and lends its lane when no story is linked yet).
        let (resolved_story, intake_lane) = match (&story_id, intake_id) {
            (Some(id), _) => (Some(id.clone()), None),
            (None, Some(intake)) => {
                let row: Option<(Option<String>, String)> = connection
                    .query_row(
                        "SELECT story_id, risk_lane FROM intake WHERE id = ?1;",
                        params![intake],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?;
                match row {
                    Some((linked, lane)) => (linked, Some(lane)),
                    None => return Err(HarnessInfraError::DoneCheckTargetMissing),
                }
            }
            (None, None) => return Err(HarnessInfraError::DoneCheckTargetMissing),
        };

        let mut checks = Vec::new();

        // Trace-link check applies to every lane and to intake-only targets.
        let trace_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM trace WHERE (?1 IS NOT NULL AND story_id = ?1)
                                          OR (?2 IS NOT NULL AND intake_id = ?2);",
            params![resolved_story, intake_id],
            |row| row.get(0),
        )?;
        checks.push(DoneCheckItem {
            label: "≥1 trace links the story/intake".to_owned(),
            passed: trace_count >= 1,
            detail: format!("{trace_count} linked trace(s)"),
        });

        let Some(story_id) = resolved_story else {
            // Intake with no linked story: only the trace-link check applies.
            return Ok(DoneCheckReport {
                target: format!("intake#{}", intake_id.unwrap_or_default()),
                lane: intake_lane.unwrap_or_else(|| "unknown".to_owned()),
                checks,
            });
        };

        let story: Option<DoneCheckStoryRow> = connection
            .query_row(
                "SELECT risk_lane, status, verify_command, last_verified_result,
                        unit_proof, integration_proof, e2e_proof, platform_proof,
                        next_action, contract_doc
                 FROM story WHERE id = ?1;",
                params![story_id],
                |row| {
                    Ok(DoneCheckStoryRow {
                        lane: row.get(0)?,
                        status: row.get(1)?,
                        verify_command: row.get(2)?,
                        last_verified: row.get(3)?,
                        unit: row.get(4)?,
                        integration: row.get(5)?,
                        e2e: row.get(6)?,
                        platform: row.get(7)?,
                        next_action: row.get(8)?,
                        contract_doc: row.get(9)?,
                    })
                },
            )
            .optional()?;
        let Some(DoneCheckStoryRow {
            lane,
            status,
            verify_command,
            last_verified,
            unit,
            integration,
            e2e,
            platform,
            next_action,
            contract_doc,
        }) = story
        else {
            return Err(HarnessInfraError::StoryNotFound(story_id));
        };

        // tiny only needs a linked trace; normal/high-risk add the proof gates.
        if lane != "tiny" {
            checks.push(DoneCheckItem {
                label: "story.status == 'implemented'".to_owned(),
                passed: status == "implemented",
                detail: format!("status = {status}"),
            });

            let has_command = verify_command
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            let verified_pass = has_command && last_verified.as_deref() == Some("pass");
            checks.push(DoneCheckItem {
                label: "verify_command set & last_verified_result == 'pass'".to_owned(),
                passed: verified_pass,
                detail: format!(
                    "command={}, last_verified={}",
                    if has_command { "yes" } else { "no" },
                    last_verified.as_deref().unwrap_or("none")
                ),
            });

            let evidence_pass: i64 = connection.query_row(
                "SELECT COUNT(*) FROM evidence
                 WHERE story_id = ?1 AND kind = 'log' AND result = 'pass';",
                params![story_id],
                |row| row.get(0),
            )?;
            checks.push(DoneCheckItem {
                label: "evidence 'log' pass row exists".to_owned(),
                passed: evidence_pass >= 1,
                detail: format!("{evidence_pass} pass log(s)"),
            });

            let proof_total = unit + integration + e2e + platform;
            checks.push(DoneCheckItem {
                label: "proof recorded (≥1 matrix flag)".to_owned(),
                passed: proof_total >= 1,
                detail: format!("unit={unit} integ={integration} e2e={e2e} plat={platform}"),
            });

            let next_cleared = next_action
                .as_deref()
                .map(str::trim)
                .map(|value| value.is_empty())
                .unwrap_or(true);
            checks.push(DoneCheckItem {
                label: "story.next_action cleared".to_owned(),
                passed: next_cleared,
                detail: next_action
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("still set: {value}"))
                    .unwrap_or_else(|| "cleared".to_owned()),
            });
        }

        if lane == "high_risk" {
            let (anchors_ok, detail) = self.high_risk_anchors_present(contract_doc.as_deref());
            checks.push(DoneCheckItem {
                label: "4 high-risk packet anchors exist".to_owned(),
                passed: anchors_ok,
                detail,
            });
        }

        Ok(DoneCheckReport {
            target: format!("story={story_id}"),
            lane,
            checks,
        })
    }

    fn audit(&self) -> Result<AuditResult> {
        let connection = self.open_existing()?;
        let mut result = AuditResult {
            orphaned_stories: audit_findings(
                &connection,
                "SELECT story.id, story.title
                 FROM story
                 LEFT JOIN trace ON trace.story_id = story.id
                 WHERE story.status IN ('planned','in_progress') AND trace.id IS NULL
                 ORDER BY story.id;",
            )?,
            unverified_stories: audit_findings(
                &connection,
                "SELECT id, title FROM story
                 WHERE verify_command IS NOT NULL
                   AND TRIM(verify_command) <> ''
                   AND last_verified_result IS NULL
                 ORDER BY id;",
            )?,
            unverified_decisions: audit_findings(
                &connection,
                "SELECT id, title FROM decision
                 WHERE verify_command IS NOT NULL
                   AND TRIM(verify_command) <> ''
                   AND last_verified_result IS NULL
                 ORDER BY id;",
            )?,
            backlog_without_outcomes: audit_findings(
                &connection,
                "SELECT CAST(id AS TEXT), title FROM backlog
                 WHERE predicted_impact IS NOT NULL
                   AND actual_outcome IS NULL
                   AND status='implemented'
                 ORDER BY id;",
            )?,
            stale_stories: audit_findings(
                &connection,
                "SELECT story.id, story.title
                 FROM story
                 JOIN trace ON trace.story_id = story.id
                 WHERE story.status <> 'implemented'
                 GROUP BY story.id, story.title
                 HAVING julianday('now') - julianday(MAX(trace.created_at)) > 30
                 ORDER BY story.id;",
            )?,
            broken_tools: Vec::new(),
        };

        let mut statement =
            connection.prepare("SELECT name, command, kind, status FROM tool ORDER BY name;")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for (name, command, kind, status) in collect_rows(rows)? {
            // Exec-probed kinds are checked live against PATH. Scanned kinds
            // (mcp/skill/http) are only "broken" once a scan has positively
            // found them missing; an un-scanned `unknown` is not drift.
            let broken = match kind.as_str() {
                "cli" | "binary" => !command_available(&self.repo_root, &command),
                _ => status == "missing",
            };
            if broken {
                result.broken_tools.push(AuditFinding {
                    id: name,
                    title: command,
                });
            }
        }
        Ok(result)
    }

    fn propose(&self, commit: bool) -> Result<Vec<ImprovementProposal>> {
        let connection = self.open_existing()?;
        let audit = self.audit()?;
        let mut proposals = Vec::new();

        for (text, count) in repeated_friction(&connection)? {
            proposals.push(ImprovementProposal {
                title: format!("Reduce repeated friction: {}", short_title(&text)),
                component: "Failure attribution".to_owned(),
                evidence: format!("{count} traces recorded similar friction: {text}"),
                predicted_impact: "Fewer repeated harness friction entries for similar tasks.".to_owned(),
                risk: "normal".to_owned(),
                suggested_action: "Update the relevant Harness docs, templates, or CLI guidance for this friction pattern.".to_owned(),
                validation_plan: "Review the next five related traces and compare friction frequency.".to_owned(),
                confidence: confidence_for_count(count),
                committed_backlog_id: None,
            });
        }

        for (key, count) in repeated_interventions(&connection)? {
            proposals.push(ImprovementProposal {
                title: format!("Address repeated intervention: {}", short_title(&key)),
                component: "Intervention recording".to_owned(),
                evidence: format!("{count} interventions share the pattern: {key}"),
                predicted_impact: "Fewer repeated human or review interventions for the same issue.".to_owned(),
                risk: "normal".to_owned(),
                suggested_action: "Clarify the relevant operating rule or validation gate that would have caught this earlier.".to_owned(),
                validation_plan: "Future interventions of this type should decrease after the rule change.".to_owned(),
                confidence: confidence_for_count(count),
                committed_backlog_id: None,
            });
        }

        for (category, count) in [
            (
                "orphaned planned or in-progress stories",
                audit.orphaned_stories.len(),
            ),
            ("unverified story commands", audit.unverified_stories.len()),
            (
                "unverified decision commands",
                audit.unverified_decisions.len(),
            ),
            (
                "implemented backlog items without outcomes",
                audit.backlog_without_outcomes.len(),
            ),
            ("stale unfinished stories", audit.stale_stories.len()),
            ("broken registered tools", audit.broken_tools.len()),
        ] {
            if count > 0 {
                proposals.push(ImprovementProposal {
                    title: format!("Clean up {category}"),
                    component: "Entropy auditing".to_owned(),
                    evidence: format!("Audit found {count} {category}."),
                    predicted_impact: "Lower entropy score and stronger completion evidence.".to_owned(),
                    risk: "tiny".to_owned(),
                    suggested_action: "Resolve the listed audit findings or record why they are intentionally retained.".to_owned(),
                    validation_plan: "Run harness-cli audit and confirm the category count decreases.".to_owned(),
                    confidence: "low".to_owned(),
                    committed_backlog_id: None,
                });
            }
        }

        if commit {
            for proposal in &mut proposals {
                connection.execute(
                    "INSERT INTO backlog (
                        title, discovered_while, current_pain, suggested_improvement,
                        risk, predicted_impact, notes
                     ) VALUES (?1, 'harness-cli propose', ?2, ?3, ?4, ?5, ?6);",
                    params![
                        proposal.title,
                        proposal.evidence,
                        proposal.suggested_action,
                        normalize_token(&proposal.risk),
                        proposal.predicted_impact,
                        format!(
                            "component: {}; confidence: {}; validation: {}",
                            proposal.component, proposal.confidence, proposal.validation_plan
                        ),
                    ],
                )?;
                proposal.committed_backlog_id = Some(connection.last_insert_rowid());
            }
        }

        Ok(proposals)
    }

    fn query_sql(&self, sql: &str) -> Result<QueryTable> {
        let connection = self.open_existing()?;
        let mut statement = connection.prepare(sql)?;
        let headers = statement
            .column_names()
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();
        let column_count = statement.column_count();
        let rows = statement.query_map([], |row| {
            let mut values = Vec::new();
            for index in 0..column_count {
                values.push(sql_value_to_string(row.get_ref(index)?));
            }
            Ok(values)
        })?;

        Ok(QueryTable {
            headers,
            rows: collect_rows(rows)?,
        })
    }
}

impl From<HarnessContext> for SqliteHarnessRepository {
    fn from(context: HarnessContext) -> Self {
        Self::new(context.repo_root, context.db_path, context.schema_dir)
    }
}

#[derive(Debug)]
struct MatrixColumns {
    story: Option<usize>,
    contract: Option<usize>,
    unit: Option<usize>,
    integration: Option<usize>,
    e2e: Option<usize>,
    platform: Option<usize>,
    status: Option<usize>,
    evidence: Option<usize>,
}

#[derive(Debug, Default)]
struct BacklogMarkdownItem {
    title: String,
    discovered_while: String,
    current_pain: String,
    suggested_improvement: String,
    risk: String,
    status: String,
}

impl MatrixColumns {
    fn from_header(fields: &[String]) -> Self {
        let mut columns = Self {
            story: None,
            contract: None,
            unit: None,
            integration: None,
            e2e: None,
            platform: None,
            status: None,
            evidence: None,
        };

        for (index, field) in fields.iter().enumerate() {
            match normalize_token(field).as_str() {
                "story" => columns.story = Some(index),
                "contract" => columns.contract = Some(index),
                "unit" => columns.unit = Some(index),
                "integration" => columns.integration = Some(index),
                "e2e" => columns.e2e = Some(index),
                "platform" => columns.platform = Some(index),
                "status" => columns.status = Some(index),
                "evidence" => columns.evidence = Some(index),
                _ => {}
            }
        }

        columns
    }
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
) -> Result<Vec<T>> {
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(HarnessInfraError::from)
}

fn trace_score_source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TraceScoreSource> {
    Ok(TraceScoreSource {
        id: row.get(0)?,
        task_summary: row.get(1)?,
        intake_id: row.get(2)?,
        risk_lane: row.get(3)?,
        agent: row.get(4)?,
        actions_taken: row.get(5)?,
        files_read: row.get(6)?,
        files_changed: row.get(7)?,
        decisions_made: row.get(8)?,
        errors: row.get(9)?,
        outcome: row.get(10)?,
        duration_seconds: row.get(11)?,
        token_estimate: row.get(12)?,
        harness_friction: row.get(13)?,
        notes: row.get(14)?,
    })
}

fn markdown_table_fields(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    trimmed
        .split('|')
        .map(|field| field.trim().to_owned())
        .collect()
}

fn field_at(fields: &[String], index: Option<usize>) -> Option<String> {
    index
        .and_then(|value| fields.get(value))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn evidence_from_fields(fields: &[String], start_index: usize) -> Option<String> {
    fields
        .get(start_index..)
        .map(|values| values.join(" | "))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn proof_from_cell(value: &str) -> i64 {
    match normalize_token(value).as_str() {
        ""
        | "no"
        | "none"
        | "n_a"
        | "na"
        | "planned"
        | "pending"
        | "blocked"
        | "not_attempted"
        | "not_operator_reviewed" => 0,
        token
            if token.starts_with("no_")
                || token.starts_with("pending")
                || token.starts_with("blocked")
                || token.contains("pending")
                || token.contains("blocked")
                || token.contains("not_attempted")
                || token.contains("not_operator_reviewed") =>
        {
            0
        }
        _ => 1,
    }
}

fn normalize_story_status(value: &str) -> String {
    match normalize_token(value).as_str() {
        "planned" => "planned",
        "in_progress" => "in_progress",
        "implemented" => "implemented",
        "changed" => "changed",
        "retired" => "retired",
        _ => "planned",
    }
    .to_owned()
}

fn normalize_decision_status(value: &str) -> String {
    let token = normalize_token(value);
    match token.as_str() {
        "proposed" => "proposed",
        "accepted" => "accepted",
        "superseded" => "superseded",
        "rejected" => "rejected",
        token if token.starts_with("superseded_") => "superseded",
        _ => "accepted",
    }
    .to_owned()
}

fn normalize_backlog_status(value: &str) -> String {
    match normalize_token(value).as_str() {
        "proposed" => "proposed",
        "accepted" => "accepted",
        "implemented" => "implemented",
        "rejected" => "rejected",
        _ => "proposed",
    }
    .to_owned()
}

fn markdown_section_first_value(content: &str, heading: &str) -> String {
    let target = format!("## {heading}");
    let mut found = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if found && !trimmed.is_empty() {
            return trimmed.to_owned();
        }
        if trimmed == target {
            found = true;
        }
    }
    String::new()
}

fn backlog_items(content: &str) -> Vec<BacklogMarkdownItem> {
    let mut in_items = false;
    let mut current_heading = String::new();
    let mut current = BacklogMarkdownItem::default();
    let mut items = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "## Items" {
            in_items = true;
            current_heading.clear();
            continue;
        }
        if !in_items {
            continue;
        }

        if let Some(heading) = trimmed.strip_prefix("### ") {
            let normalized = normalize_token(heading);
            if normalized == "title" && !current.title.is_empty() {
                items.push(current);
                current = BacklogMarkdownItem::default();
            }
            current_heading = normalized;
            continue;
        }

        if trimmed.is_empty() || current_heading.is_empty() {
            continue;
        }

        let target = match current_heading.as_str() {
            "title" => &mut current.title,
            "discovered_while" => &mut current.discovered_while,
            "current_pain" => &mut current.current_pain,
            "suggested_improvement" => &mut current.suggested_improvement,
            "risk" => &mut current.risk,
            "status" => &mut current.status,
            _ => continue,
        };
        if target.is_empty() {
            *target = trimmed.to_owned();
        }
    }

    if !current.title.is_empty() {
        items.push(current);
    }
    items
}

fn empty_to_none(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn command_available(repo_root: &Path, command: &str) -> bool {
    let first = command.split_whitespace().next().unwrap_or(command);
    if first.is_empty() {
        return false;
    }
    let candidate = Path::new(first);
    if candidate.is_absolute() {
        return candidate.exists();
    }
    if first.contains('/') || first.contains('\\') {
        return repo_root.join(first).exists();
    }
    env::var_os("PATH")
        .is_some_and(|path| env::split_paths(&path).any(|dir| dir.join(first).exists()))
}

/// Kind-aware presence probe. Returns `(status, detail)` where status is one of
/// `present` / `missing` / `unknown`. It never fails: an absent extension is a
/// fact to report, not an error to raise.
fn scan_tool_status(
    repo_root: &Path,
    kind: &str,
    command: &str,
    scan_target: Option<&str>,
) -> (&'static str, String) {
    match kind {
        "cli" | "binary" => {
            if command_available(repo_root, command) {
                ("present", command.to_owned())
            } else {
                ("missing", command.to_owned())
            }
        }
        "mcp" | "skill" => match scan_target.map(str::trim).filter(|t| !t.is_empty()) {
            Some(target) => {
                if scan_target_resolves(repo_root, target) {
                    ("present", target.to_owned())
                } else {
                    ("missing", target.to_owned())
                }
            }
            None => (
                "unknown",
                "no scan target; agent confirms availability".to_owned(),
            ),
        },
        "http" => match scan_target.map(str::trim).filter(|t| !t.is_empty()) {
            Some(target) => {
                if http_reachable(target) || scan_target_resolves(repo_root, target) {
                    ("present", target.to_owned())
                } else {
                    ("missing", target.to_owned())
                }
            }
            None => ("unknown", "no scan target".to_owned()),
        },
        _ => ("unknown", String::new()),
    }
}

/// Resolve a declarative scan target as a filesystem path: `~` expands to HOME,
/// absolute paths are tested directly, relative paths are tested against the
/// repo root.
fn scan_target_resolves(repo_root: &Path, target: &str) -> bool {
    let expanded = expand_home(target);
    let path = Path::new(&expanded);
    if path.is_absolute() {
        path.exists()
    } else {
        repo_root.join(&expanded).exists()
    }
}

fn expand_home(target: &str) -> String {
    if let Some(rest) = target.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return format!("{}/{}", home.to_string_lossy(), rest);
        }
    }
    target.to_owned()
}

/// Best-effort TCP reachability for `http`/`https` scan targets. Any failure
/// (parse, DNS, timeout, refused) is reported as not reachable rather than an
/// error, so a down endpoint degrades the capability instead of breaking intake.
fn http_reachable(target: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let (default_port, rest) = if let Some(rest) = target.strip_prefix("https://") {
        (443u16, rest)
    } else if let Some(rest) = target.strip_prefix("http://") {
        (80u16, rest)
    } else {
        return false;
    };

    let authority = rest.split('/').next().unwrap_or("");
    if authority.is_empty() {
        return false;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port.parse::<u16>().unwrap_or(default_port)),
        None => (authority, default_port),
    };

    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return false;
    };
    addresses
        .into_iter()
        .any(|address| TcpStream::connect_timeout(&address, Duration::from_secs(2)).is_ok())
}

fn tool_args_json(args: &[ToolArgSpec]) -> Option<String> {
    if args.is_empty() {
        return None;
    }
    Some(format!(
        "[{}]",
        args.iter()
            .map(|arg| {
                format!(
                    "{{\"name\":\"{}\",\"type\":\"{}\",\"required\":{},\"help\":\"{}\"}}",
                    escape_json(&arg.name),
                    escape_json(&arg.arg_type),
                    arg.required,
                    escape_json(arg.help.as_deref().unwrap_or(""))
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    ))
}

fn parse_stored_tool_args(value: Option<&str>) -> Vec<ToolArgSpec> {
    let Some(value) = value else {
        return Vec::new();
    };
    if !value.contains("\"name\"") {
        return Vec::new();
    }
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split("},{")
        .filter_map(|raw| {
            let item = raw.trim_matches('{').trim_matches('}');
            let name = json_object_value(item, "name")?;
            let arg_type = json_object_value(item, "type").unwrap_or_else(|| "string".to_owned());
            let required = json_object_value(item, "required")
                .map(|value| value == "true")
                .unwrap_or(false);
            let help = json_object_value(item, "help").filter(|value| !value.is_empty());
            Some(ToolArgSpec {
                name,
                arg_type,
                required,
                help,
            })
        })
        .collect()
}

fn json_object_value(raw: &str, key: &str) -> Option<String> {
    let target = format!("\"{key}\":");
    let start = raw.find(&target)? + target.len();
    let rest = &raw[start..];
    if let Some(rest) = rest.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(rest[..end].to_owned())
    } else {
        Some(rest.split(',').next().unwrap_or_default().trim().to_owned())
    }
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn audit_findings(connection: &Connection, sql: &str) -> Result<Vec<AuditFinding>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], |row| {
        Ok(AuditFinding {
            id: row.get(0)?,
            title: row.get(1)?,
        })
    })?;
    collect_rows(rows)
}

fn repeated_friction(connection: &Connection) -> Result<Vec<(String, usize)>> {
    let mut statement = connection.prepare(
        "SELECT harness_friction FROM trace
         WHERE harness_friction IS NOT NULL
           AND TRIM(harness_friction) <> ''
           AND LOWER(TRIM(harness_friction)) <> 'none';",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let values = collect_rows(rows)?;
    Ok(repeated_values(values))
}

fn repeated_interventions(connection: &Connection) -> Result<Vec<(String, usize)>> {
    let mut statement = connection.prepare(
        "SELECT type || ': ' || description FROM intervention
         WHERE TRIM(description) <> '';",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let values = collect_rows(rows)?;
    Ok(repeated_values(values))
}

fn repeated_values(values: Vec<String>) -> Vec<(String, usize)> {
    let mut grouped: Vec<(String, String, usize)> = Vec::new();
    for value in values {
        let key = normalize_token(&value);
        if let Some(existing) = grouped.iter_mut().find(|item| item.0 == key) {
            existing.2 += 1;
        } else {
            grouped.push((key, value, 1));
        }
    }
    grouped
        .into_iter()
        .filter(|(_, _, count)| *count >= 2)
        .map(|(_, value, count)| (value, count))
        .collect()
}

fn confidence_for_count(count: usize) -> String {
    if count >= 3 {
        "high".to_owned()
    } else {
        "medium".to_owned()
    }
}

fn short_title(value: &str) -> String {
    let words = value
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    if words.len() > 72 {
        format!("{}...", &words[..69])
    } else {
        words
    }
}

fn verifier_shell() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    }
}

/// Human-readable scope label for a recap (e.g. `story=US-006`, `epic=US-00`).
fn recap_scope(filter: &RecapFilter) -> String {
    let mut parts = Vec::new();
    if let Some(story) = filter.story_id.as_deref() {
        parts.push(format!("story={story}"));
    }
    if let Some(prefix) = filter.epic_prefix.as_deref() {
        parts.push(format!("epic={prefix}"));
    }
    if let Some(since) = filter.since.as_deref() {
        parts.push(format!("since={since}"));
    }
    if parts.is_empty() {
        "all".to_owned()
    } else {
        parts.join(" · ")
    }
}

/// Attribute a friction string to one of the 11 Components by substring match,
/// falling back to "Unattributed". Deterministic: first declared match wins.
fn attribute_friction_component(text: &str) -> String {
    let lower = text.to_lowercase();
    for responsibility in RESPONSIBILITIES {
        if lower.contains(&responsibility.to_lowercase()) {
            return (*responsibility).to_owned();
        }
    }
    "Unattributed".to_owned()
}

/// Turn a count map into a deterministically ordered list: count desc, then key
/// ascending, capped to `limit` entries.
fn top_counts(counts: BTreeMap<String, i64>, limit: usize) -> Vec<RecapCount> {
    let mut items: Vec<RecapCount> = counts
        .into_iter()
        .map(|(key, count)| RecapCount { key, count })
        .collect();
    items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    items.truncate(limit);
    items
}

/// Make a path segment filesystem-safe: keep alphanumerics and `-_.`,
/// replacing everything else with `_`.
fn sanitize_key(value: &str) -> String {
    let cleaned: String = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "item".to_owned()
    } else {
        cleaned
    }
}

fn is_decision_file_name(file_name: &str) -> bool {
    let Some((prefix, _)) = file_name.split_once('-') else {
        return false;
    };
    prefix.len() == 4 && prefix.chars().all(|character| character.is_ascii_digit())
}

const KNOWLEDGE_IGNORE_DIRS: &[&str] = &["target", "node_modules", "dist", "build", "vendor"];
/// Gitignored runtime stores excluded from the knowledge map by exact path.
const KNOWLEDGE_IGNORE_PATHS: &[&str] = &["_harness/evidence"];
const KNOWLEDGE_PYTHON_ARTIFACT_DIRS: &[&str] = &[
    "__pycache__",
    ".pytest_cache",
    ".ruff_cache",
    ".mypy_cache",
    "htmlcov",
];
const KNOWLEDGE_WALK_MAX_DEPTH: usize = 4;

/// Filesystem gateway for the Knowledge Index. Reads repo structure and tech
/// signals and reads/writes `docs/KNOWLEDGE_INDEX.md`. Holds no SQLite state.
#[derive(Debug)]
pub struct KnowledgeWorkspace {
    repo_root: PathBuf,
}

impl KnowledgeWorkspace {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    fn index_path(&self) -> PathBuf {
        self.repo_root.join(knowledge::INDEX_PATH)
    }

    /// Ensure the index's parent directory exists so it is always listed as a
    /// top-level entry (the index lives under it), keeping scaffold idempotent.
    pub fn ensure_index_dir(&self) -> Result<()> {
        if let Some(parent) = self.index_path().parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    pub fn read_existing(&self) -> Result<Option<String>> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(fs::read_to_string(path)?))
    }

    pub fn write_index(&self, content: &str) -> Result<PathBuf> {
        let path = self.index_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        Ok(path)
    }

    pub fn gather(&self) -> Result<KnowledgeInputs> {
        let repo_name = self
            .repo_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("repository")
            .to_owned();

        let mut entries: Vec<TopLevelEntry> = Vec::new();
        let mut signals: BTreeSet<String> = BTreeSet::new();

        for entry in fs::read_dir(&self.repo_root)? {
            let entry = entry?;
            let name = match entry.file_name().to_str() {
                Some(name) => name.to_owned(),
                None => continue,
            };
            // `DirEntry::file_type` does not follow symlinks, so a symlink to a
            // directory would otherwise be reported as a file. Resolve through
            // the path so a linked directory is listed with a trailing slash.
            let is_dir = entry.path().is_dir();

            // Every top-level name is a detection signal (dotfiles included).
            signals.insert(name.clone());

            // The structure listing skips hidden, build, Python-generated, and
            // local-db noise. Directory ignores still keep a regular file that
            // happens to share a name (e.g. `build`) listed.
            let ignored = is_ignored_knowledge_entry(&name, is_dir);
            if !ignored {
                entries.push(TopLevelEntry { name, is_dir });
            }
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));

        self.collect_signals(&mut signals);

        let subdirectories = self.collect_subdirectories(&entries);
        let commands = self.collect_commands();
        let technologies = knowledge::detect_technologies(&signals);
        Ok(KnowledgeInputs {
            repo_name,
            technologies,
            entries,
            subdirectories,
            commands,
        })
    }

    /// List the immediate subdirectories of each top-level directory (one
    /// level deeper than `entries`), addressed by relative path. Hidden,
    /// ignored, and db-artifact names are skipped.
    fn collect_subdirectories(&self, entries: &[TopLevelEntry]) -> Vec<TopLevelEntry> {
        let mut subdirectories: Vec<TopLevelEntry> = Vec::new();
        for parent in entries.iter().filter(|entry| entry.is_dir) {
            let Ok(read) = fs::read_dir(self.repo_root.join(&parent.name)) else {
                continue;
            };
            for entry in read.flatten() {
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if !entry.path().is_dir() {
                    continue;
                }
                if is_ignored_knowledge_entry(&name, true) {
                    continue;
                }
                let relative = format!("{}/{}", parent.name, name);
                // The evidence store is a gitignored runtime artifact (decision
                // 0002), not repo structure — keep it out of the knowledge map.
                if KNOWLEDGE_IGNORE_PATHS.contains(&relative.as_str()) {
                    continue;
                }
                subdirectories.push(TopLevelEntry {
                    name: relative,
                    is_dir: true,
                });
            }
        }
        subdirectories.sort_by(|left, right| left.name.cmp(&right.name));
        subdirectories
    }

    /// Derive deterministic build/test/run commands from root manifests.
    fn collect_commands(&self) -> Vec<RunCommand> {
        let mut commands: Vec<RunCommand> = Vec::new();
        let mut push = |command: &str, label: &str| {
            if !commands.iter().any(|item| item.command == command) {
                commands.push(RunCommand {
                    command: command.to_owned(),
                    label: label.to_owned(),
                });
            }
        };
        let read_root = |name: &str| fs::read_to_string(self.repo_root.join(name)).ok();

        if self.repo_root.join("Cargo.toml").exists() {
            push("cargo build", "build");
            push("cargo test", "test");
        }
        if let Some(text) = read_root("package.json") {
            for script in ["build", "test", "dev", "start", "lint"] {
                if package_json_has_script(&text, script) {
                    push(&format!("npm run {script}"), script);
                }
            }
        }
        if let Some(text) = read_root("Makefile") {
            for target in ["build", "test", "run", "lint"] {
                if makefile_has_target(&text, target) {
                    push(&format!("make {target}"), target);
                }
            }
        }
        if self.repo_root.join("go.mod").exists() {
            push("go build ./...", "build");
            push("go test ./...", "test");
        }
        let python_manifest = read_root("pyproject.toml")
            .into_iter()
            .chain(read_root("requirements.txt"))
            .collect::<String>()
            .to_lowercase();
        if python_manifest.contains("pytest") {
            push("pytest", "test");
        }
        commands
    }

    fn collect_signals(&self, signals: &mut BTreeSet<String>) {
        let mut has_rusqlite = false;
        let mut stack: Vec<(PathBuf, usize)> = vec![(self.repo_root.clone(), 0)];
        while let Some((dir, depth)) = stack.pop() {
            let Ok(read) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in read.flatten() {
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() {
                    if is_hidden(&name) || is_ignored_knowledge_dir(&name) {
                        continue;
                    }
                    if depth + 1 < KNOWLEDGE_WALK_MAX_DEPTH {
                        stack.push((entry.path(), depth + 1));
                    }
                    continue;
                }
                if is_db_artifact(&name) || is_python_generated_file(&name) {
                    continue;
                }
                if let Some(extension) = std::path::Path::new(&name)
                    .extension()
                    .and_then(|value| value.to_str())
                {
                    signals.insert(format!("ext:{}", extension.to_lowercase()));
                }
                match name.as_str() {
                    "Cargo.toml" => {
                        if let Ok(text) = fs::read_to_string(entry.path()) {
                            if text.contains("[workspace]") {
                                signals.insert(knowledge::SIGNAL_CARGO_WORKSPACE.to_owned());
                            }
                            if text.contains("rusqlite") {
                                has_rusqlite = true;
                            }
                        }
                    }
                    "package.json" => {
                        if let Ok(text) = fs::read_to_string(entry.path()) {
                            collect_node_framework_signals(&text, signals);
                        }
                    }
                    "requirements.txt" | "pyproject.toml" => {
                        if let Ok(text) = fs::read_to_string(entry.path()) {
                            collect_python_framework_signals(&text, signals);
                        }
                    }
                    "Gemfile" => {
                        if let Ok(text) = fs::read_to_string(entry.path()) {
                            if text.to_lowercase().contains("rails") {
                                signals.insert("dep:rails".to_owned());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if has_rusqlite {
            signals.insert(knowledge::SIGNAL_RUST_SQLITE.to_owned());
        }
    }
}

/// Emit `dep:*` signals for frameworks named in a `package.json`. Quoted
/// dependency names keep the substring match from firing on prose.
fn collect_node_framework_signals(text: &str, signals: &mut BTreeSet<String>) {
    let markers = [
        ("\"react\"", "dep:react"),
        ("\"next\"", "dep:next"),
        ("\"vue\"", "dep:vue"),
        ("\"@angular/", "dep:angular"),
        ("\"svelte\"", "dep:svelte"),
        ("\"express\"", "dep:express"),
        ("\"@nestjs/", "dep:nestjs"),
    ];
    for (needle, signal) in markers {
        if text.contains(needle) {
            signals.insert(signal.to_owned());
        }
    }
}

/// Emit `dep:*` signals for Python web frameworks named in a manifest.
fn collect_python_framework_signals(text: &str, signals: &mut BTreeSet<String>) {
    let lowered = text.to_lowercase();
    for (needle, signal) in [
        ("django", "dep:django"),
        ("flask", "dep:flask"),
        ("fastapi", "dep:fastapi"),
    ] {
        if lowered.contains(needle) {
            signals.insert(signal.to_owned());
        }
    }
}

/// True when a `package.json` `scripts` block defines `"<name>":`.
fn package_json_has_script(text: &str, name: &str) -> bool {
    let Some(scripts_start) = text.find("\"scripts\"") else {
        return false;
    };
    let after = &text[scripts_start..];
    let Some(open) = after.find('{') else {
        return false;
    };
    let block = &after[open..];
    let end = block.find('}').unwrap_or(block.len());
    block[..end].contains(&format!("\"{name}\""))
}

/// True when a `Makefile` declares a `<name>:` target at column zero.
fn makefile_has_target(text: &str, name: &str) -> bool {
    let prefix = format!("{name}:");
    text.lines().any(|line| line.starts_with(&prefix))
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn is_ignored_dir(name: &str) -> bool {
    KNOWLEDGE_IGNORE_DIRS.contains(&name)
}

fn is_ignored_knowledge_dir(name: &str) -> bool {
    is_ignored_dir(name) || is_python_generated_dir(name)
}

fn is_ignored_knowledge_entry(name: &str, is_dir: bool) -> bool {
    is_hidden(name)
        || is_db_artifact(name)
        || if is_dir {
            is_ignored_knowledge_dir(name)
        } else {
            is_python_generated_file(name)
        }
}

fn is_python_generated_dir(name: &str) -> bool {
    KNOWLEDGE_PYTHON_ARTIFACT_DIRS.contains(&name) || name.ends_with(".egg-info")
}

fn is_python_generated_file(name: &str) -> bool {
    matches!(name, ".coverage") || name.ends_with(".pyc") || name.ends_with(".pyo")
}

fn is_db_artifact(name: &str) -> bool {
    // Match the SQLite database and its WAL/SHM sidecars (the sidecars exist
    // only while a connection is open — e.g. when a verify command runs while
    // the CLI holds the db — and must not be reported as repo structure).
    name.ends_with(".db") || name.ends_with(".db-wal") || name.ends_with(".db-shm")
}

fn sql_value_to_string(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),
        ValueRef::Blob(value) => format!("<{} bytes>", value.len()),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::application::{
        BacklogAddInput, BacklogCloseInput, DecisionAddInput, IntakeInput, InterventionAddInput,
        InterventionFilter, StoryAddInput, StoryUpdateInput, ToolRegisterInput, TraceInput,
    };
    use crate::domain::{BacklogFilter, BoolFlag, CsvList, InputType, RiskLane, TraceQualityTier};

    fn test_repository() -> (TempDir, SqliteHarnessRepository) {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf();
        let repository = SqliteHarnessRepository::new(
            repo_root.clone(),
            temp_dir.path().join("harness.db"),
            repo_root.join("_harness/schema"),
        );
        (temp_dir, repository)
    }

    fn story_columns(connection: &Connection) -> Vec<String> {
        let mut statement = connection.prepare("PRAGMA table_info(story);").unwrap();
        let rows = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap();
        rows.collect::<std::result::Result<Vec<_>, _>>().unwrap()
    }

    #[test]
    fn init_creates_database_and_schema() {
        let (_temp_dir, repository) = test_repository();

        let result = repository.init().unwrap();

        assert!(matches!(result, InitResult::Created { .. }));
        assert_eq!(repository.query_stats().unwrap().intakes, 0);
        let connection = repository.open_existing().unwrap();
        let schema_version = SqliteHarnessRepository::schema_version(&connection).unwrap();
        assert_eq!(schema_version, 7);
        let story_columns = story_columns(&connection);
        assert!(story_columns.contains(&"verify_command".to_owned()));
        assert!(story_columns.contains(&"last_verified_at".to_owned()));
        assert!(story_columns.contains(&"last_verified_result".to_owned()));
        assert!(story_columns.contains(&"next_action".to_owned()));
    }

    #[test]
    fn migrate_applies_story_verify_columns_to_existing_database() {
        let (_temp_dir, repository) = test_repository();
        let connection = repository.open_or_create().unwrap();
        repository.apply_schema_v1(&connection).unwrap();
        drop(connection);

        let result = repository.migrate().unwrap();

        assert_eq!(result.current_version, 1);
        assert_eq!(result.applied, vec![2, 3, 4, 5, 6, 7]);
        let connection = repository.open_existing().unwrap();
        assert_eq!(
            SqliteHarnessRepository::schema_version(&connection).unwrap(),
            7
        );
        let story_columns = story_columns(&connection);
        assert!(story_columns.contains(&"verify_command".to_owned()));
        assert!(story_columns.contains(&"last_verified_at".to_owned()));
        assert!(story_columns.contains(&"last_verified_result".to_owned()));
    }

    #[test]
    fn migration_005_backfills_kind_from_command_prefix() {
        let (_temp_dir, repository) = test_repository();
        let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .join("_harness/schema");

        // Build a pre-kind (v4) database: v1 base plus migrations 002-004 only.
        let connection = repository.open_or_create().unwrap();
        repository.apply_schema_v1(&connection).unwrap();
        for file in [
            "002-story-verify.sql",
            "003-tool-registry.sql",
            "004-intervention.sql",
        ] {
            let sql = std::fs::read_to_string(schema_dir.join(file)).unwrap();
            connection.execute_batch(&sql).unwrap();
        }
        assert_eq!(
            SqliteHarnessRepository::schema_version(&connection).unwrap(),
            4
        );

        // Insert tools the old way (no kind column existed yet).
        for (name, command) in [
            ("mcp-example", "mcp:example-server"),
            ("skill-example", "skill:example-skill"),
            ("cli-example", "./deploy.sh"),
        ] {
            connection
                .execute(
                    "INSERT INTO tool (name, command, description, responsibility)
                     VALUES (?1, ?2, 'pre-kind registered tool example', 'Verification');",
                    params![name, command],
                )
                .unwrap();
        }
        drop(connection);

        // Upgrade: migration 005 must infer kind from the command prefix.
        // (006/007 also apply now; 005 is the one under test.)
        assert_eq!(repository.migrate().unwrap().applied, vec![5, 6, 7]);
        let connection = repository.open_existing().unwrap();
        let kind_of = |name: &str| -> String {
            connection
                .query_row(
                    "SELECT kind FROM tool WHERE name=?1;",
                    params![name],
                    |row| row.get::<_, String>(0),
                )
                .unwrap()
        };
        assert_eq!(kind_of("mcp-example"), "mcp");
        assert_eq!(kind_of("skill-example"), "skill");
        assert_eq!(kind_of("cli-example"), "cli");
    }

    #[test]
    fn records_and_queries_intake() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();

        let id = repository
            .record_intake(IntakeInput {
                input_type: InputType::HarnessImprovement,
                summary: "Port one CLI slice".to_owned(),
                risk_lane: RiskLane::HighRisk,
                risk_flags: CsvList::from_optional(Some("public contracts".to_owned())),
                affected_docs: CsvList::from_optional(None),
                story_id: Some("US-002".to_owned()),
                notes: None,
            })
            .unwrap();

        let intakes = repository.query_intakes().unwrap();
        assert_eq!(id, 1);
        assert_eq!(intakes[0].summary, "Port one CLI slice");
        assert_eq!(intakes[0].input_type, "harness_improvement");
        assert_eq!(intakes[0].risk_lane, "high_risk");

        let connection = repository.open_existing().unwrap();
        let missing_lists_are_null: (bool, bool) = connection
            .query_row(
                "SELECT risk_flags IS NULL, affected_docs IS NULL FROM intake WHERE id=?1;",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(missing_lists_are_null, (false, true));
    }

    #[test]
    fn decision_verify_runs_from_repo_root() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_root).unwrap();
        let schema_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf()
            .join("_harness/schema");
        let repository = SqliteHarnessRepository::new(
            repo_root.clone(),
            temp_dir.path().join("harness.db"),
            schema_root,
        );
        repository.init().unwrap();

        let pwd_output = repo_root.join("verify-pwd.txt");
        let verify_command = if cfg!(windows) {
            "cd > verify-pwd.txt".to_owned()
        } else {
            "pwd > verify-pwd.txt".to_owned()
        };
        repository
            .add_decision(DecisionAddInput {
                id: "0001-test".to_owned(),
                title: "Verify from root".to_owned(),
                status: "accepted".to_owned(),
                doc_path: None,
                verify_command: Some(verify_command),
                predicted_impact: None,
                notes: None,
            })
            .unwrap();

        let result = repository.verify_decision("0001-test").unwrap();

        assert_eq!(result.result, "pass");
        assert_eq!(
            fs::canonicalize(fs::read_to_string(pwd_output).unwrap().trim()).unwrap(),
            fs::canonicalize(repo_root).unwrap()
        );
    }

    #[test]
    fn story_add_update_and_verify_status_store_verify_command() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();

        repository
            .add_story(StoryAddInput {
                id: "US-VERIFY".to_owned(),
                title: "Verify command story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: Some("echo ok".to_owned()),
                notes: None,
            })
            .unwrap();
        assert_eq!(
            repository
                .story_verify_status("US-VERIFY")
                .unwrap()
                .verify_command
                .as_deref(),
            Some("echo ok")
        );

        repository
            .update_story(StoryUpdateInput {
                id: "US-VERIFY".to_owned(),
                status: None,
                evidence: None,
                unit: None,
                integration: None,
                e2e: None,
                platform: None,
                verify_command: Some("npm test".to_owned()),
                next_action: None,
            })
            .unwrap();

        assert_eq!(
            repository
                .story_verify_status("US-VERIFY")
                .unwrap()
                .verify_command
                .as_deref(),
            Some("npm test")
        );
    }

    fn story_next_action(repository: &SqliteHarnessRepository, id: &str) -> Option<String> {
        let connection = repository.open_existing().unwrap();
        connection
            .query_row(
                "SELECT next_action FROM story WHERE id=?1;",
                params![id],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap()
    }

    fn next_action_trace(outcome: &str, next_action: Option<&str>) -> TraceInput {
        TraceInput {
            task_summary: format!("trace {outcome}"),
            intake_id: None,
            story_id: Some("US-NA".to_owned()),
            agent: None,
            outcome: Some(outcome.to_owned()),
            duration_seconds: None,
            token_estimate: None,
            friction: None,
            notes: None,
            next_action: next_action.map(str::to_owned),
            actions: CsvList::from_optional(None),
            files_read: CsvList::from_optional(None),
            files_changed: CsvList::from_optional(None),
            decisions: CsvList::from_optional(None),
            errors: CsvList::from_optional(None),
        }
    }

    fn temp_repo_repository() -> (TempDir, PathBuf, SqliteHarnessRepository) {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_root).unwrap();
        let schema_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf()
            .join("_harness/schema");
        let repository = SqliteHarnessRepository::new(
            repo_root.clone(),
            temp_dir.path().join("harness.db"),
            schema_root,
        );
        (temp_dir, repo_root, repository)
    }

    #[test]
    fn evidence_add_hashes_copies_and_lists() {
        let (_temp_dir, repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();
        repository
            .add_story(StoryAddInput {
                id: "US-EV".to_owned(),
                title: "Evidence story".to_owned(),
                risk_lane: RiskLane::HighRisk,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();

        let artifact = repo_root.join("sample.log");
        fs::write(&artifact, b"hello evidence\nsecond line\n").unwrap();
        let expected_sha = evidence::sha256_hex(b"hello evidence\nsecond line\n");

        let result = repository
            .add_evidence(EvidenceAddInput {
                kind: "log".to_owned(),
                path: "sample.log".to_owned(),
                story_id: Some("US-EV".to_owned()),
                trace_id: None,
                command: Some("cargo test".to_owned()),
                source: "agent".to_owned(),
                result: Some("pass".to_owned()),
                notes: None,
            })
            .unwrap();

        assert_eq!(result.sha256, expected_sha);
        assert_eq!(result.bytes, 27);
        assert!(!result.deduped);
        assert!(repo_root.join(&result.path).exists());
        assert!(result.path.starts_with("_harness/evidence/US-EV/"));

        let listed = repository
            .list_evidence(EvidenceFilter {
                story_id: Some("US-EV".to_owned()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].sha256, expected_sha);
        assert_eq!(listed[0].result.as_deref(), Some("pass"));

        // Identical content under the same anchor dedups (no second row).
        let again = repository
            .add_evidence(EvidenceAddInput {
                kind: "log".to_owned(),
                path: "sample.log".to_owned(),
                story_id: Some("US-EV".to_owned()),
                trace_id: None,
                command: None,
                source: "agent".to_owned(),
                result: None,
                notes: None,
            })
            .unwrap();
        assert!(again.deduped);
        assert_eq!(again.id, result.id);
        assert_eq!(
            repository
                .list_evidence(EvidenceFilter {
                    story_id: Some("US-EV".to_owned()),
                    ..Default::default()
                })
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn db_sidecars_are_treated_as_db_artifacts() {
        // WAL/SHM sidecars appear while a verify command runs with the db open;
        // they must not be reported as new repo structure by knowledge check.
        assert!(is_db_artifact("harness.db"));
        assert!(is_db_artifact("harness.db-wal"));
        assert!(is_db_artifact("harness.db-shm"));
        assert!(!is_db_artifact("notes.md"));
    }

    #[test]
    fn done_check_is_lane_aware() {
        let (_temp_dir, _repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();

        // tiny: a single linked trace is enough.
        repository
            .add_story(StoryAddInput {
                id: "US-TINY".to_owned(),
                title: "Tiny story".to_owned(),
                risk_lane: RiskLane::Tiny,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();
        repository
            .record_trace(TraceInput {
                task_summary: "tiny work done".to_owned(),
                intake_id: None,
                story_id: Some("US-TINY".to_owned()),
                agent: None,
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: None,
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(None),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();
        let tiny = repository
            .done_check(Some("US-TINY".to_owned()), None)
            .unwrap();
        assert_eq!(tiny.checks.len(), 1);
        assert!(tiny.passed());

        // normal with nothing done fails.
        repository
            .add_story(StoryAddInput {
                id: "US-NORM".to_owned(),
                title: "Normal story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: Some("echo ok".to_owned()),
                notes: None,
            })
            .unwrap();
        let unfinished = repository
            .done_check(Some("US-NORM".to_owned()), None)
            .unwrap();
        assert!(!unfinished.passed());

        // Drive every normal gate to green.
        repository
            .record_trace(TraceInput {
                task_summary: "implemented normal story".to_owned(),
                intake_id: None,
                story_id: Some("US-NORM".to_owned()),
                agent: None,
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: None,
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(None),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();
        repository.verify_story("US-NORM", true).unwrap();
        repository
            .update_story(StoryUpdateInput {
                id: "US-NORM".to_owned(),
                status: Some("implemented".to_owned()),
                evidence: None,
                unit: Some(BoolFlag(1)),
                integration: None,
                e2e: None,
                platform: None,
                verify_command: None,
                next_action: None,
            })
            .unwrap();
        let finished = repository
            .done_check(Some("US-NORM".to_owned()), None)
            .unwrap();
        assert!(
            finished.passed(),
            "all normal gates should be green: {:?}",
            finished.checks
        );
    }

    #[test]
    fn query_recap_rolls_up_outcomes_files_and_friction() {
        let (_temp_dir, _repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();
        repository
            .add_story(StoryAddInput {
                id: "US-RC".to_owned(),
                title: "Recap story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();

        let trace = |outcome: &str, files: Option<&str>, friction: Option<&str>, next: Option<&str>| TraceInput {
            task_summary: format!("recap {outcome}"),
            intake_id: None,
            story_id: Some("US-RC".to_owned()),
            agent: None,
            outcome: Some(outcome.to_owned()),
            duration_seconds: None,
            token_estimate: None,
            friction: friction.map(str::to_owned),
            notes: None,
            next_action: next.map(str::to_owned),
            actions: CsvList::from_optional(None),
            files_read: CsvList::from_optional(None),
            files_changed: CsvList::from_optional(files.map(str::to_owned)),
            decisions: CsvList::from_optional(None),
            errors: CsvList::from_optional(None),
        };

        repository
            .record_trace(trace(
                "completed",
                Some("src/foo.rs,src/bar.rs"),
                Some("none"),
                None,
            ))
            .unwrap();
        repository
            .record_trace(trace(
                "partial",
                Some("src/foo.rs"),
                Some("Spec unclear. Attribution: Task specification."),
                Some("finish schema"),
            ))
            .unwrap();
        repository
            .record_trace(trace("completed", Some("src/foo.rs"), Some("none"), None))
            .unwrap();

        let recap = repository
            .query_recap(RecapFilter {
                story_id: Some("US-RC".to_owned()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(recap.trace_count, 3);
        assert_eq!(recap.completed, 2);
        assert_eq!(recap.partial, 1);
        // src/foo.rs touched in all three traces -> ranked first.
        assert_eq!(recap.files[0].key, "src/foo.rs");
        assert_eq!(recap.files[0].count, 3);
        // Friction attributed to a real Component (one of the 11).
        assert_eq!(recap.friction.len(), 1);
        assert_eq!(recap.friction[0].key, "Task specification");
        assert_eq!(recap.friction[0].count, 1);

        // Determinism: identical db -> identical rollup.
        let again = repository
            .query_recap(RecapFilter {
                story_id: Some("US-RC".to_owned()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(recap, again);
    }

    #[test]
    fn query_status_handles_empty_db_and_caps_sections() {
        let (_temp_dir, _repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();

        // Empty db: every section is empty and the call does not crash.
        let empty = repository.query_status(StatusFilter::default()).unwrap();
        assert_eq!(empty.active.total, 0);
        assert!(empty.active.items.is_empty());
        assert_eq!(empty.needs_proof.total, 0);
        assert_eq!(empty.resume.total, 0);
        assert_eq!(empty.recent.total, 0);

        // Six in-progress stories with --limit 5 report total=6, 5 shown, 1 hidden.
        for index in 0..6 {
            repository
                .add_story(StoryAddInput {
                    id: format!("US-{index:03}"),
                    title: format!("Story {index}"),
                    risk_lane: RiskLane::Normal,
                    contract_doc: None,
                    verify_command: None,
                    notes: None,
                })
                .unwrap();
            repository
                .update_story(StoryUpdateInput {
                    id: format!("US-{index:03}"),
                    status: Some("in_progress".to_owned()),
                    evidence: None,
                    unit: None,
                    integration: None,
                    e2e: None,
                    platform: None,
                    verify_command: None,
                    next_action: None,
                })
                .unwrap();
        }
        let capped = repository
            .query_status(StatusFilter {
                lane: None,
                limit: Some(5),
            })
            .unwrap();
        assert_eq!(capped.active.total, 6);
        assert_eq!(capped.active.items.len(), 5);
        assert_eq!(capped.active.hidden(), 1);

        // --full removes the cap.
        let full = repository
            .query_status(StatusFilter {
                lane: None,
                limit: None,
            })
            .unwrap();
        assert_eq!(full.active.items.len(), 6);

        // Lane filter narrows story-derived sections.
        let high = repository
            .query_status(StatusFilter {
                lane: Some("high_risk".to_owned()),
                limit: Some(5),
            })
            .unwrap();
        assert_eq!(high.active.total, 0);
    }

    #[test]
    fn query_status_resume_ignores_stale_unfinished_traces_after_completion() {
        let (_temp_dir, _repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();
        repository
            .add_story(StoryAddInput {
                id: "US-NA".to_owned(),
                title: "Next action".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();

        repository
            .record_trace(next_action_trace("partial", Some("finish the parser")))
            .unwrap();
        repository
            .record_trace(next_action_trace("completed", None))
            .unwrap();

        let status = repository.query_status(StatusFilter::default()).unwrap();
        assert_eq!(status.resume.total, 0);
        assert!(status.resume.items.is_empty());
    }

    #[test]
    fn verify_auto_capture_keeps_last_and_keeps_fail_then_pass() {
        let (_temp_dir, _repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();
        let set_cmd = |command: &str| {
            repository
                .update_story(StoryUpdateInput {
                    id: "US-CAP".to_owned(),
                    status: None,
                    evidence: None,
                    unit: None,
                    integration: None,
                    e2e: None,
                    platform: None,
                    verify_command: Some(command.to_owned()),
                    next_action: None,
                })
                .unwrap();
        };
        let logs = || {
            repository
                .list_evidence(EvidenceFilter {
                    story_id: Some("US-CAP".to_owned()),
                    kind: Some("log".to_owned()),
                    ..Default::default()
                })
                .unwrap()
        };

        repository
            .add_story(StoryAddInput {
                id: "US-CAP".to_owned(),
                title: "Capture story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: Some("exit 1".to_owned()),
                notes: None,
            })
            .unwrap();

        // First fail captures one log row with result=fail.
        let first = repository.verify_story("US-CAP", true).unwrap();
        assert_eq!(first.result, "fail");
        assert!(first.evidence_id.is_some());
        assert_eq!(logs().len(), 1);
        let first_id = first.evidence_id.unwrap();

        // A different failing output replaces the prior fail row (keep-last).
        set_cmd("echo changed; exit 1");
        repository.verify_story("US-CAP", true).unwrap();
        let after_replace = logs();
        assert_eq!(after_replace.len(), 1);
        assert_ne!(after_replace[0].id, first_id);

        // Transition fail -> pass keeps BOTH rows (the fix evidence survives).
        set_cmd("echo ok");
        let pass = repository.verify_story("US-CAP", true).unwrap();
        assert_eq!(pass.result, "pass");
        let after_pass = logs();
        assert_eq!(after_pass.len(), 2);
        assert_eq!(
            after_pass
                .iter()
                .filter(|r| r.result.as_deref() == Some("pass"))
                .count(),
            1
        );
        assert_eq!(
            after_pass
                .iter()
                .filter(|r| r.result.as_deref() == Some("fail"))
                .count(),
            1
        );

        // Re-running the same passing command dedups by content (no new row).
        repository.verify_story("US-CAP", true).unwrap();
        assert_eq!(logs().len(), 2);

        // --no-capture records no evidence and adds no rows.
        let no_cap = repository.verify_story("US-CAP", false).unwrap();
        assert!(no_cap.evidence_id.is_none());
        assert_eq!(logs().len(), 2);
    }

    #[test]
    fn capture_keeps_both_when_identical_output_transitions_fail_to_pass() {
        // Regression: dedup must be scoped by result. Byte-identical log text
        // captured first as fail then as pass must yield TWO rows (one each),
        // so done-check's pass-log gate can be satisfied.
        let (_temp_dir, _repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();
        repository
            .add_story(StoryAddInput {
                id: "US-IDENT".to_owned(),
                title: "Identical-output story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();

        let connection = repository.open_existing().unwrap();
        let same_log = "$ run-tests\nfixed banner, same bytes\n";
        repository
            .capture_verify_log(&connection, "US-IDENT", "run-tests", "fail", same_log)
            .unwrap();
        repository
            .capture_verify_log(&connection, "US-IDENT", "run-tests", "pass", same_log)
            .unwrap();
        drop(connection);

        let logs = repository
            .list_evidence(EvidenceFilter {
                story_id: Some("US-IDENT".to_owned()),
                kind: Some("log".to_owned()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(logs.len(), 2, "fail and pass logs must both survive");
        assert_eq!(
            logs.iter()
                .filter(|r| r.result.as_deref() == Some("pass"))
                .count(),
            1
        );
    }

    #[test]
    fn evidence_add_requires_anchor_and_existing_file() {
        let (_temp_dir, repo_root, repository) = temp_repo_repository();
        repository.init().unwrap();

        // Anchor that does not exist, with a file that DOES exist: must fail
        // with a clear anchor error and must NOT leave an artifact behind.
        let artifact = repo_root.join("orphan.log");
        fs::write(&artifact, b"orphan candidate\n").unwrap();
        let bad_anchor = repository
            .add_evidence(EvidenceAddInput {
                kind: "log".to_owned(),
                path: "orphan.log".to_owned(),
                story_id: Some("US-NOPE".to_owned()),
                trace_id: None,
                command: None,
                source: "agent".to_owned(),
                result: None,
                notes: None,
            })
            .unwrap_err();
        assert!(matches!(
            bad_anchor,
            HarnessInfraError::EvidenceAnchorNotFound(_)
        ));
        assert!(
            !repo_root.join("_harness/evidence").exists(),
            "no artifact should be copied for a bad anchor"
        );

        let no_anchor = repository
            .add_evidence(EvidenceAddInput {
                kind: "log".to_owned(),
                path: "missing.log".to_owned(),
                story_id: None,
                trace_id: None,
                command: None,
                source: "agent".to_owned(),
                result: None,
                notes: None,
            })
            .unwrap_err();
        assert!(matches!(
            no_anchor,
            HarnessInfraError::EvidenceValidation(
                evidence::EvidenceValidationError::MissingAnchor
            )
        ));

        let missing_file = repository
            .add_evidence(EvidenceAddInput {
                kind: "log".to_owned(),
                path: "missing.log".to_owned(),
                story_id: Some("US-X".to_owned()),
                trace_id: None,
                command: None,
                source: "agent".to_owned(),
                result: None,
                notes: None,
            })
            .unwrap_err();
        assert!(matches!(
            missing_file,
            HarnessInfraError::EvidenceArtifactMissing(_)
        ));
    }

    #[test]
    fn next_action_is_enforced_for_unfinished_and_cleared_on_completed() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();
        repository
            .add_story(StoryAddInput {
                id: "US-NA".to_owned(),
                title: "Next-action story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();

        // partial/blocked/failed without --next-action is rejected.
        for outcome in ["partial", "blocked", "failed"] {
            let error = repository
                .record_trace(next_action_trace(outcome, None))
                .unwrap_err();
            assert!(matches!(error, HarnessInfraError::NextActionRequired(_)));
        }

        // partial WITH next-action writes the live story pointer.
        repository
            .record_trace(next_action_trace("partial", Some("finish the parser")))
            .unwrap();
        assert_eq!(
            story_next_action(&repository, "US-NA").as_deref(),
            Some("finish the parser")
        );

        // completed clears the live pointer.
        repository
            .record_trace(next_action_trace("completed", None))
            .unwrap();
        assert_eq!(story_next_action(&repository, "US-NA"), None);

        // story update can set the live pointer directly.
        repository
            .update_story(StoryUpdateInput {
                id: "US-NA".to_owned(),
                status: None,
                evidence: None,
                unit: None,
                integration: None,
                e2e: None,
                platform: None,
                verify_command: None,
                next_action: Some("review with reviewer".to_owned()),
            })
            .unwrap();
        assert_eq!(
            story_next_action(&repository, "US-NA").as_deref(),
            Some("review with reviewer")
        );
    }

    #[test]
    fn story_verify_records_pass_fail_and_missing_command() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_root).unwrap();
        let schema_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf()
            .join("_harness/schema");
        let repository = SqliteHarnessRepository::new(
            repo_root.clone(),
            temp_dir.path().join("harness.db"),
            schema_root,
        );
        repository.init().unwrap();

        let pwd_output = repo_root.join("story-verify-pwd.txt");
        let verify_command = if cfg!(windows) {
            "cd > story-verify-pwd.txt".to_owned()
        } else {
            "pwd > story-verify-pwd.txt".to_owned()
        };
        repository
            .add_story(StoryAddInput {
                id: "US-PASS".to_owned(),
                title: "Passing story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: Some(verify_command),
                notes: None,
            })
            .unwrap();
        let pass = repository.verify_story("US-PASS", false).unwrap();
        assert_eq!(pass.result, "pass");
        assert_eq!(
            fs::canonicalize(fs::read_to_string(pwd_output).unwrap().trim()).unwrap(),
            fs::canonicalize(repo_root).unwrap()
        );
        assert_eq!(
            repository
                .story_verify_status("US-PASS")
                .unwrap()
                .last_verified_result
                .as_deref(),
            Some("pass")
        );

        repository
            .add_story(StoryAddInput {
                id: "US-FAIL".to_owned(),
                title: "Failing story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: Some("exit 1".to_owned()),
                notes: None,
            })
            .unwrap();
        let fail = repository.verify_story("US-FAIL", false).unwrap();
        assert_eq!(fail.result, "fail");
        assert_eq!(
            repository
                .story_verify_status("US-FAIL")
                .unwrap()
                .last_verified_result
                .as_deref(),
            Some("fail")
        );

        repository
            .add_story(StoryAddInput {
                id: "US-MISSING".to_owned(),
                title: "Missing command story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();
        assert!(matches!(
            repository.verify_story("US-MISSING", false),
            Err(HarnessInfraError::MissingStoryVerifyCommand(id)) if id == "US-MISSING"
        ));
    }

    #[test]
    fn story_verify_all_reports_pass_fail_and_skipped() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();
        for (id, command) in [
            ("US-PASS", Some("exit 0")),
            ("US-FAIL", Some("exit 1")),
            ("US-SKIP", None),
        ] {
            repository
                .add_story(StoryAddInput {
                    id: id.to_owned(),
                    title: id.to_owned(),
                    risk_lane: RiskLane::Normal,
                    contract_doc: None,
                    verify_command: command.map(str::to_owned),
                    notes: None,
                })
                .unwrap();
        }

        let result = repository.verify_all_stories().unwrap();

        assert_eq!(result.passed(), 1);
        assert_eq!(result.failed(), 1);
        assert_eq!(result.skipped(), 1);
        assert_eq!(
            repository
                .story_verify_status("US-PASS")
                .unwrap()
                .last_verified_result
                .as_deref(),
            Some("pass")
        );
        assert_eq!(
            repository
                .story_verify_status("US-FAIL")
                .unwrap()
                .last_verified_result
                .as_deref(),
            Some("fail")
        );
    }

    #[test]
    fn tool_registry_register_query_and_remove_work() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();

        repository
            .register_tool(ToolRegisterInput {
                name: "deploy-check".to_owned(),
                command: "definitely-missing-tool".to_owned(),
                description: "Verify deploy health before release".to_owned(),
                responsibility: "Verification".to_owned(),
                args: Vec::new(),
                force: true,
                kind: "cli".to_owned(),
                capability: Some("deploy-verification".to_owned()),
                scan_target: None,
            })
            .unwrap();
        assert!(matches!(
            repository.register_tool(ToolRegisterInput {
                name: "deploy-check".to_owned(),
                command: "definitely-missing-tool".to_owned(),
                description: "Verify deploy health before release".to_owned(),
                responsibility: "Verification".to_owned(),
                args: Vec::new(),
                force: true,
                kind: "cli".to_owned(),
                capability: Some("deploy-verification".to_owned()),
                scan_target: None,
            }),
            Err(HarnessInfraError::ToolAlreadyExists(_, _))
        ));

        let verification_tools = repository
            .query_tools(Some("Verification".to_owned()), None)
            .unwrap();
        assert!(verification_tools
            .iter()
            .any(|tool| tool.name == "deploy-check" && tool.source == "registered"));

        // Capability lookup returns the registered provider.
        let by_capability = repository
            .query_tools(None, Some("deploy-verification".to_owned()))
            .unwrap();
        assert!(by_capability.iter().any(|tool| tool.name == "deploy-check"));

        repository.remove_tool("deploy-check").unwrap();
        assert!(!repository
            .query_tools(None, None)
            .unwrap()
            .iter()
            .any(|tool| tool.name == "deploy-check"));
    }

    #[test]
    fn tool_check_scans_and_persists_status_per_kind() {
        let (temp_dir, repository) = test_repository();
        repository.init().unwrap();

        // Absolute scan targets keep the test hermetic: test_repository's
        // repo_root points at the real project, so relative targets would
        // resolve against the checkout rather than the temp dir.
        let present_target = temp_dir.path().join("skill-present");
        std::fs::create_dir_all(&present_target).unwrap();
        let missing_target = temp_dir.path().join("mcp-missing");

        // An mcp tool whose scan target does not exist -> missing.
        repository
            .register_tool(ToolRegisterInput {
                name: "mcp-example".to_owned(),
                command: "mcp:example-server".to_owned(),
                description: "Example MCP-backed provider".to_owned(),
                responsibility: "Verification".to_owned(),
                args: Vec::new(),
                force: false,
                kind: "mcp".to_owned(),
                capability: Some("impact-analysis".to_owned()),
                scan_target: Some(missing_target.to_string_lossy().into_owned()),
            })
            .unwrap();

        // A skill tool whose scan target exists -> present.
        repository
            .register_tool(ToolRegisterInput {
                name: "skill-example".to_owned(),
                command: "skill:example-skill".to_owned(),
                description: "Example skill-backed provider".to_owned(),
                responsibility: "Verification".to_owned(),
                args: Vec::new(),
                force: false,
                kind: "skill".to_owned(),
                capability: Some("impact-analysis".to_owned()),
                scan_target: Some(present_target.to_string_lossy().into_owned()),
            })
            .unwrap();

        let results = repository.check_tools(None).unwrap();
        let mcp_tool = results.iter().find(|r| r.name == "mcp-example").unwrap();
        let skill_tool = results.iter().find(|r| r.name == "skill-example").unwrap();
        assert_eq!(mcp_tool.status, "missing");
        assert_eq!(skill_tool.status, "present");

        // Status is persisted, not just returned.
        let stored = repository
            .query_tools(None, Some("impact-analysis".to_owned()))
            .unwrap();
        assert_eq!(stored.len(), 2);
        assert!(stored
            .iter()
            .all(|tool| tool.checked_at.as_deref().is_some_and(|v| !v.is_empty())));
        assert_eq!(
            stored
                .iter()
                .find(|t| t.name == "skill-example")
                .unwrap()
                .status,
            "present"
        );
    }

    #[test]
    fn interventions_can_be_added_and_filtered() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();
        repository
            .add_story(StoryAddInput {
                id: "US-I".to_owned(),
                title: "Intervention story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();
        let trace_id = repository
            .record_trace(TraceInput {
                task_summary: "Trace for intervention".to_owned(),
                intake_id: None,
                story_id: Some("US-I".to_owned()),
                agent: Some("codex".to_owned()),
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: Some("none".to_owned()),
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(None),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();
        repository
            .add_intervention(InterventionAddInput {
                trace_id: Some(trace_id),
                story_id: Some("US-I".to_owned()),
                intervention_type: "correction".to_owned(),
                description: "Use error handling instead of unwrap".to_owned(),
                source: "human".to_owned(),
                impact: Some("Reduced panic risk".to_owned()),
            })
            .unwrap();

        assert_eq!(
            repository
                .query_interventions(InterventionFilter {
                    trace_id: Some(trace_id),
                    story_id: None,
                    intervention_type: None,
                })
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repository
                .query_interventions(InterventionFilter {
                    trace_id: None,
                    story_id: Some("US-I".to_owned()),
                    intervention_type: Some("override".to_owned()),
                })
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn audit_detects_drift_and_propose_can_commit_backlog_items() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();
        repository
            .add_story(StoryAddInput {
                id: "US-AUDIT".to_owned(),
                title: "Audit story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: Some("exit 0".to_owned()),
                notes: None,
            })
            .unwrap();
        repository
            .update_story(StoryUpdateInput {
                id: "US-AUDIT".to_owned(),
                status: Some("in_progress".to_owned()),
                evidence: None,
                unit: None,
                integration: None,
                e2e: None,
                platform: None,
                verify_command: None,
                next_action: None,
            })
            .unwrap();
        repository
            .add_backlog(BacklogAddInput {
                title: "Implemented without outcome".to_owned(),
                discovered_while: None,
                current_pain: None,
                suggestion: None,
                risk: Some(RiskLane::Tiny),
                predicted_impact: Some("Expected improvement".to_owned()),
                notes: None,
            })
            .unwrap();
        repository
            .close_backlog(BacklogCloseInput {
                id: 1,
                status: "implemented".to_owned(),
                actual_outcome: None,
            })
            .unwrap();
        repository
            .register_tool(ToolRegisterInput {
                name: "missing-tool".to_owned(),
                command: "definitely-missing-tool".to_owned(),
                description: "Missing command for audit coverage".to_owned(),
                responsibility: "Verification".to_owned(),
                args: Vec::new(),
                force: true,
                kind: "cli".to_owned(),
                capability: None,
                scan_target: None,
            })
            .unwrap();
        for _ in 0..2 {
            repository
                .record_trace(TraceInput {
                    task_summary: "Repeated friction trace".to_owned(),
                    intake_id: None,
                    story_id: None,
                    agent: Some("codex".to_owned()),
                    outcome: Some("completed".to_owned()),
                    duration_seconds: None,
                    token_estimate: None,
                    friction: Some("Context rules missed schema decision".to_owned()),
                    notes: None,
                    next_action: None,
                    actions: CsvList::from_optional(Some("read".to_owned())),
                    files_read: CsvList::from_optional(Some("_harness/docs/HARNESS.md".to_owned())),
                    files_changed: CsvList::from_optional(Some(
                        "_harness/schema/003-tool-registry.sql".to_owned(),
                    )),
                    decisions: CsvList::from_optional(None),
                    errors: CsvList::from_optional(None),
                })
                .unwrap();
        }

        let audit = repository.audit().unwrap();
        assert_eq!(audit.orphaned_stories.len(), 1);
        assert_eq!(audit.unverified_stories.len(), 1);
        assert_eq!(audit.backlog_without_outcomes.len(), 1);
        assert_eq!(audit.broken_tools.len(), 1);
        assert!(audit.entropy_score() > 0);

        let proposals = repository.propose(true).unwrap();
        assert!(proposals.iter().any(|proposal| proposal
            .evidence
            .contains("Context rules missed schema decision")));
        assert!(proposals
            .iter()
            .all(|proposal| proposal.committed_backlog_id.is_some()));
        assert!(repository.query_backlog(BacklogFilter::Open).unwrap().len() >= 1);
    }

    #[test]
    fn story_backlog_trace_and_queries_work() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();

        repository
            .add_story(StoryAddInput {
                id: "US-T".to_owned(),
                title: "Test story".to_owned(),
                risk_lane: RiskLane::Normal,
                contract_doc: None,
                verify_command: None,
                notes: None,
            })
            .unwrap();
        repository
            .update_story(StoryUpdateInput {
                id: "US-T".to_owned(),
                status: Some("implemented".to_owned()),
                evidence: Some("unit test".to_owned()),
                unit: Some(BoolFlag(1)),
                integration: None,
                e2e: None,
                platform: None,
                verify_command: None,
                next_action: None,
            })
            .unwrap();
        assert_eq!(repository.query_matrix().unwrap()[0].unit, 1);

        let backlog_id = repository
            .add_backlog(BacklogAddInput {
                title: "Improve CLI".to_owned(),
                discovered_while: None,
                current_pain: Some("manual SQL".to_owned()),
                suggestion: None,
                risk: Some(RiskLane::HighRisk),
                predicted_impact: None,
                notes: None,
            })
            .unwrap();
        repository
            .close_backlog(BacklogCloseInput {
                id: backlog_id,
                status: "implemented".to_owned(),
                actual_outcome: Some("done".to_owned()),
            })
            .unwrap();
        assert_eq!(
            repository.query_backlog(BacklogFilter::All).unwrap()[0]
                .actual_outcome
                .as_deref(),
            Some("done")
        );

        let trace_id = repository
            .record_trace(TraceInput {
                task_summary: "Test trace".to_owned(),
                intake_id: None,
                story_id: Some("US-T".to_owned()),
                agent: Some("test".to_owned()),
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: Some("none".to_owned()),
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(Some("one,two".to_owned())),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();
        assert_eq!(trace_id, 1);
        assert_eq!(
            repository.query_traces().unwrap()[0].task_summary,
            "Test trace"
        );
        assert_eq!(
            repository.query_friction().unwrap()[0].harness_friction,
            "none"
        );
    }

    #[test]
    fn friction_query_includes_intake_context_and_filters_null_friction() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();
        let intake_id = repository
            .record_intake(IntakeInput {
                input_type: InputType::ChangeRequest,
                summary: "Friction query context".to_owned(),
                risk_lane: RiskLane::Normal,
                risk_flags: CsvList::from_optional(None),
                affected_docs: CsvList::from_optional(None),
                story_id: None,
                notes: None,
            })
            .unwrap();
        repository
            .record_trace(TraceInput {
                task_summary: "Trace without friction".to_owned(),
                intake_id: Some(intake_id),
                story_id: None,
                agent: Some("codex".to_owned()),
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: None,
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(None),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();
        repository
            .record_trace(TraceInput {
                task_summary: "Trace with linked friction".to_owned(),
                intake_id: Some(intake_id),
                story_id: None,
                agent: Some("codex".to_owned()),
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: Some("Linked friction".to_owned()),
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(None),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();
        repository
            .record_trace(TraceInput {
                task_summary: "Trace with unlinked friction".to_owned(),
                intake_id: None,
                story_id: None,
                agent: Some("codex".to_owned()),
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: Some("Unlinked friction".to_owned()),
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(None),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();

        let friction = repository.query_friction().unwrap();

        assert_eq!(friction.len(), 2);
        assert_eq!(friction[0].risk_lane, None);
        assert_eq!(friction[0].input_type, None);
        assert_eq!(friction[1].risk_lane.as_deref(), Some("normal"));
        assert_eq!(friction[1].input_type.as_deref(), Some("change_request"));
    }

    #[test]
    fn import_brownfield_seeds_markdown_state_idempotently() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("repo");
        fs::create_dir_all(repo_root.join("docs/decisions")).unwrap();
        fs::create_dir_all(repo_root.join("_harness/docs")).unwrap();
        fs::write(
            repo_root.join("_harness/docs/TEST_MATRIX.md"),
            r#"# Test Matrix

| Story | Contract | Unit | Integration | E2E | Platform | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| US-010 | docs/product/tasks.md | yes | pending | no | mac smoke | implemented | cargo test |
"#,
        )
        .unwrap();
        fs::write(
            repo_root.join("docs/decisions/0007-test-decision.md"),
            r#"# Test Decision

## Status

Accepted
"#,
        )
        .unwrap();
        fs::write(
            repo_root.join("_harness/docs/HARNESS_BACKLOG.md"),
            r#"# Harness Backlog

## Items

### Title

Import existing docs

### Discovered While

Testing brownfield import

### Current Pain

Existing Harness v0 repos have markdown truth.

### Suggested Improvement

Seed the durable database.

### Risk

normal

### Status

accepted

### Title

Keep installer checksum

### Discovered While

Testing release install

### Current Pain

Downloads need verification.

### Suggested Improvement

Verify sha256 files.

### Risk

high-risk

### Status

implemented
"#,
        )
        .unwrap();

        let source_repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf();
        let repository = SqliteHarnessRepository::new(
            repo_root.clone(),
            temp_dir.path().join("harness.db"),
            source_repo_root.join("_harness/schema"),
        );
        repository.init().unwrap();

        let first = repository.import_brownfield().unwrap();
        let second = repository.import_brownfield().unwrap();

        assert_eq!(
            first,
            BrownfieldImportResult {
                stories: 1,
                decisions: 1,
                backlog_items: 2,
            }
        );
        assert_eq!(second.backlog_items, 2);

        let matrix = repository.query_matrix().unwrap();
        assert_eq!(matrix[0].id, "US-010");
        assert_eq!(matrix[0].title, "docs/product/tasks.md");
        assert_eq!(matrix[0].status, "implemented");
        assert_eq!(matrix[0].unit, 1);
        assert_eq!(matrix[0].integration, 0);
        assert_eq!(matrix[0].platform, 1);

        let decisions = repository.query_decisions().unwrap();
        assert_eq!(decisions[0].id, "0007-test-decision");
        assert_eq!(decisions[0].status, "accepted");

        let backlog = repository.query_backlog(BacklogFilter::All).unwrap();
        assert_eq!(backlog.len(), 2);
        assert!(backlog
            .iter()
            .any(|item| item.title == "Import existing docs"
                && item.status == "accepted"
                && item.risk.as_deref() == Some("normal")));
        assert!(backlog
            .iter()
            .any(|item| item.title == "Keep installer checksum"
                && item.status == "implemented"
                && item.risk.as_deref() == Some("high_risk")));
    }

    #[test]
    fn filters_open_and_closed_backlog_items() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();

        let proposed_id = repository
            .add_backlog(BacklogAddInput {
                title: "Proposed item".to_owned(),
                discovered_while: None,
                current_pain: None,
                suggestion: None,
                risk: Some(RiskLane::Tiny),
                predicted_impact: Some("Should improve trace review.".to_owned()),
                notes: None,
            })
            .unwrap();
        let implemented_id = repository
            .add_backlog(BacklogAddInput {
                title: "Implemented item".to_owned(),
                discovered_while: None,
                current_pain: None,
                suggestion: None,
                risk: Some(RiskLane::Normal),
                predicted_impact: Some("Should reduce missing proof.".to_owned()),
                notes: None,
            })
            .unwrap();
        repository
            .close_backlog(BacklogCloseInput {
                id: implemented_id,
                status: "implemented".to_owned(),
                actual_outcome: Some("Proof gaps were found earlier.".to_owned()),
            })
            .unwrap();

        let all = repository.query_backlog(BacklogFilter::All).unwrap();
        let open = repository.query_backlog(BacklogFilter::Open).unwrap();
        let closed = repository.query_backlog(BacklogFilter::Closed).unwrap();

        assert_eq!(all.len(), 2);
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, proposed_id);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].id, implemented_id);
        assert_eq!(
            closed[0].actual_outcome.as_deref(),
            Some("Proof gaps were found earlier.")
        );
    }

    #[test]
    fn scores_latest_and_specific_trace_with_lane_lookup() {
        let (_temp_dir, repository) = test_repository();
        repository.init().unwrap();
        let intake_id = repository
            .record_intake(IntakeInput {
                input_type: InputType::HarnessImprovement,
                summary: "High risk trace quality test".to_owned(),
                risk_lane: RiskLane::HighRisk,
                risk_flags: CsvList::from_optional(None),
                affected_docs: CsvList::from_optional(None),
                story_id: None,
                notes: None,
            })
            .unwrap();
        let first_trace = repository
            .record_trace(TraceInput {
                task_summary: "Minimal trace test".to_owned(),
                intake_id: None,
                story_id: None,
                agent: None,
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: None,
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(None),
                files_read: CsvList::from_optional(None),
                files_changed: CsvList::from_optional(None),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();
        repository
            .record_trace(TraceInput {
                task_summary: "Standard trace linked to high risk intake".to_owned(),
                intake_id: Some(intake_id),
                story_id: None,
                agent: Some("codex".to_owned()),
                outcome: Some("completed".to_owned()),
                duration_seconds: None,
                token_estimate: None,
                friction: Some("none".to_owned()),
                notes: None,
                next_action: None,
                actions: CsvList::from_optional(Some("read,patched".to_owned())),
                files_read: CsvList::from_optional(Some("PHASE3.md".to_owned())),
                files_changed: CsvList::from_optional(Some(
                    "crates/harness-cli/src/domain.rs".to_owned(),
                )),
                decisions: CsvList::from_optional(None),
                errors: CsvList::from_optional(None),
            })
            .unwrap();

        let latest = repository.score_trace(None).unwrap();
        assert_eq!(latest.achieved, TraceQualityTier::Standard);
        assert_eq!(latest.required, Some(TraceQualityTier::Detailed));
        assert!(!latest.meets_requirement);
        assert!(latest
            .missing_detailed
            .iter()
            .any(|field| field.starts_with("decisions_made")));

        let specific = repository.score_trace(Some(first_trace)).unwrap();
        assert_eq!(specific.trace_id, first_trace);
        assert_eq!(specific.achieved, TraceQualityTier::Minimal);
        assert_eq!(specific.required, None);
        assert!(specific.meets_requirement);
    }

    #[test]
    fn knowledge_workspace_gathers_structure_and_tech() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("demo");
        fs::create_dir_all(repo_root.join("src")).unwrap();
        fs::create_dir_all(repo_root.join("target/debug")).unwrap();
        fs::write(
            repo_root.join("Cargo.toml"),
            "[workspace]\nmembers=[\"x\"]\n[dependencies]\nrusqlite=\"0\"\n",
        )
        .unwrap();
        fs::write(repo_root.join("schema.sql"), "CREATE TABLE t(x);").unwrap();
        fs::write(repo_root.join("harness.db"), "binary").unwrap();
        fs::write(repo_root.join(".prettierrc"), "{}").unwrap();

        let workspace = KnowledgeWorkspace::new(repo_root);
        let inputs = workspace.gather().unwrap();

        // Build/db artifacts and dotfiles are excluded from the structure list.
        let names: Vec<&str> = inputs.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Cargo.toml", "schema.sql", "src"]);
        assert!(!names.contains(&"target"));
        assert!(!names.contains(&"harness.db"));
        // Dotfile is excluded from the listing but still drives detection.
        assert!(inputs.technologies.contains(&"Rust".to_owned()));
        assert!(inputs.technologies.contains(&"Cargo Workspace".to_owned()));
        assert!(inputs.technologies.contains(&"SQLite".to_owned()));
        assert!(inputs.technologies.contains(&"Prettier".to_owned()));
    }

    #[test]
    fn gather_collects_subdirectories_commands_and_frameworks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("app");
        fs::create_dir_all(repo_root.join("src/components")).unwrap();
        fs::create_dir_all(repo_root.join("src/lib")).unwrap();
        fs::create_dir_all(repo_root.join("node_modules/react")).unwrap();
        fs::write(
            repo_root.join("package.json"),
            "{\n  \"dependencies\": { \"react\": \"^18\", \"next\": \"^14\" },\n  \
             \"scripts\": { \"build\": \"next build\", \"test\": \"vitest\" }\n}\n",
        )
        .unwrap();
        fs::write(repo_root.join("yarn.lock"), "").unwrap();

        let inputs = KnowledgeWorkspace::new(repo_root).gather().unwrap();

        // Frameworks and the package manager are read from manifest contents.
        for expected in ["Node.js", "React", "Next.js", "Yarn"] {
            assert!(
                inputs.technologies.iter().any(|t| t == expected),
                "expected {expected} in {:?}",
                inputs.technologies
            );
        }

        // Immediate subdirectories are listed by path; ignored dirs excluded.
        let subdirs: Vec<&str> = inputs
            .subdirectories
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(subdirs, vec!["src/components", "src/lib"]);
        assert!(!subdirs.iter().any(|s| s.contains("node_modules")));

        // Commands are derived from package.json scripts.
        let commands: Vec<&str> = inputs.commands.iter().map(|c| c.command.as_str()).collect();
        assert!(commands.contains(&"npm run build"));
        assert!(commands.contains(&"npm run test"));
        assert!(!commands.contains(&"npm run dev"));
    }

    #[test]
    fn gather_ignores_generated_python_artifacts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("demo");
        fs::create_dir_all(repo_root.join("data/__pycache__")).unwrap();
        fs::create_dir_all(repo_root.join(".pytest_cache")).unwrap();
        fs::create_dir_all(repo_root.join(".ruff_cache")).unwrap();
        fs::create_dir_all(repo_root.join("htmlcov")).unwrap();
        fs::create_dir_all(repo_root.join("package.egg-info")).unwrap();
        fs::create_dir_all(repo_root.join("src")).unwrap();
        fs::write(
            repo_root.join("pyproject.toml"),
            "[project]\nname=\"demo\"\n",
        )
        .unwrap();
        fs::write(repo_root.join("module.pyc"), "").unwrap();
        fs::write(repo_root.join("module.pyo"), "").unwrap();
        fs::write(repo_root.join(".coverage"), "").unwrap();

        let inputs = KnowledgeWorkspace::new(repo_root).gather().unwrap();
        let names: Vec<&str> = inputs.entries.iter().map(|e| e.name.as_str()).collect();
        let subdirs: Vec<&str> = inputs
            .subdirectories
            .iter()
            .map(|e| e.name.as_str())
            .collect();

        assert_eq!(names, vec!["data", "pyproject.toml", "src"]);
        assert!(subdirs.is_empty(), "__pycache__ should not be listed");
        assert!(!names.contains(&"htmlcov"));
        assert!(!names.contains(&"package.egg-info"));
        assert!(!names.contains(&"module.pyc"));
        assert!(!names.contains(&"module.pyo"));
    }

    #[test]
    fn gather_lists_files_named_like_ignored_dirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("demo");
        fs::create_dir_all(repo_root.join("target")).unwrap();
        // A regular file sharing an ignored directory name must still be listed.
        fs::write(repo_root.join("build"), "#!/bin/sh\n").unwrap();
        fs::write(repo_root.join("Cargo.toml"), "[package]\nname=\"d\"\n").unwrap();

        let workspace = KnowledgeWorkspace::new(repo_root);
        let inputs = workspace.gather().unwrap();
        let names: Vec<&str> = inputs.entries.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"build"), "file `build` should be listed");
        assert!(!names.contains(&"target"), "dir `target` should be ignored");
    }

    #[cfg(unix)]
    #[test]
    fn gather_marks_symlinked_directory_as_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path().join("demo");
        fs::create_dir_all(repo_root.join("real")).unwrap();
        fs::write(repo_root.join("Cargo.toml"), "[package]\nname=\"d\"\n").unwrap();
        std::os::unix::fs::symlink(repo_root.join("real"), repo_root.join("linked")).unwrap();

        let workspace = KnowledgeWorkspace::new(repo_root);
        let inputs = workspace.gather().unwrap();
        let linked = inputs
            .entries
            .iter()
            .find(|entry| entry.name == "linked")
            .expect("symlink should be listed");

        // A symlink pointing at a directory is reported as a directory.
        assert!(linked.is_dir);
    }
}
