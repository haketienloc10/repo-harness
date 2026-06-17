# Harness Components

This taxonomy maps the current `repository-harness` repository to two component
frameworks used by Phase 2 and updated by Phase 3 active observability work:

- Runtime Substrate responsibilities: the 11 responsibility areas the harness
  should cover.
- NexAU decomposition: the seven implementation surfaces that influence agent
  behavior.

Status values:

- **Covered**: the repository has an explicit file, command, or record for this
  responsibility.
- **Partial**: the repository has some support, but the support is incomplete,
  manual, or not yet measured.
- **Missing**: no meaningful support exists yet.

## Responsibility Map

| #   | Responsibility         | Status  | Harness Files                                                                                                                                                                                                                                                                                                                                                       | Evidence                                                                                                                                                                                                                             | Gap                                                                                           |
| --- | ---------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| 1   | Task specification     | Covered | `AGENTS.md`, `_harness/docs/FEATURE_INTAKE.md`, `_harness/docs/templates/story.md`, `_harness/docs/templates/spec-intake.md`, `_harness/docs/templates/high-risk-story/*`, `docs/stories/*`, `intake` table, `story` table                                                                                                                                                                              | Requests are classified by type and lane before implementation; normal and high-risk work have templates and durable story rows.                                                                                                     | Keep story packets synchronized with future product docs.                                     |
| 2   | Context selection      | Covered | `AGENTS.md`, `_harness/docs/CONTEXT_RULES.md`, `_harness/docs/ARCHITECTURE.md`, `docs/decisions/*`, `docs/product/README.md`, `_harness/bin/harness-cli score-context`                                                                                                                                                                                                                 | Phase 2 adds phase-by-lane context rules and retrieval triggers; Phase 5 adds context scoring against recorded trace reads.                                                                                                          | Future automation could enforce context selection instead of only measuring it.               |
| 3   | Tool access            | Covered | `_harness/bin/harness-cli`, `_harness/docs/TOOL_REGISTRY.md`, `tool` table, `crates/harness-cli/*`, `scripts/install-harness.sh`, `scripts/build-harness-cli-release.sh`                                                                                                                                                                                                      | The Harness CLI exposes operational commands and a machine-readable tool manifest through `query tools`; external tools can be registered and removed.                                                                               | Permission profiles and usage analytics remain future work.                                   |
| 4   | Project memory         | Covered | `_harness/docs/HARNESS.md`, `docs/decisions/*`, `_harness/docs/GLOSSARY.md`, `_harness/docs/HARNESS_BACKLOG.md`, `docs/stories/*`, `harness.db`, `decision`, `backlog`, and `trace` tables                                                                                                                                                                                                     | Decisions, backlog, stories, and traces preserve durable knowledge across tasks.                                                                                                                                                     | Future work should add staleness checks and summarize old traces.                             |
| 5   | Task state             | Covered | `_harness/bin/harness-cli query matrix`, `_harness/bin/harness-cli query status`, `_harness/docs/TEST_MATRIX.md`, `intake` table, `story` table, `trace` table, `story.next_action`/`trace.next_action` (migration 006)                                                                                                                                                          | Durable records track intake, story status, proof columns, and task traces; `next_action` carries WIP across sessions and surfaces in `query status` RESUME.                                                                          | Lifecycle checks now exist via `done-check`; benchmark of resume continuity is future work.   |
| 6   | Observability          | Partial | `_harness/docs/TRACE_SPEC.md`, `docs/product/read-model.md`, `trace` table, `_harness/bin/harness-cli trace`, `_harness/bin/harness-cli score-trace`, `_harness/bin/harness-cli query traces`, `_harness/bin/harness-cli query friction`, `_harness/bin/harness-cli query status`, `_harness/bin/harness-cli query recap`, `_harness/docs/HARNESS_MATURITY.md`                              | Traces are auto-scored when recorded; the Read-Model (`query status`/`query recap`) gives a deterministic ranked snapshot of being/been/to-do plus per-story rollup over traces.                                                      | Read-Model is a derived view; benchmark/attribution ingestion (full H3) remains future work.  |
| 7   | Failure attribution    | Partial | `_harness/docs/HARNESS_COMPONENTS.md`, `_harness/docs/TRACE_SPEC.md`, `trace.errors`, `trace.harness_friction`, `_harness/docs/HARNESS_BACKLOG.md`, `backlog` table, `_harness/bin/harness-cli query friction`                                                                                                                                                                                  | Failures can be tied to files, components, friction, backlog proposals, and linked intake lane/type context.                                                                                                                         | No automated attribution from benchmark failures to harness components exists yet.            |
| 8   | Verification           | Covered | `_harness/docs/TEST_MATRIX.md`, `_harness/bin/harness-cli query matrix`, `_harness/bin/harness-cli story verify`, `_harness/bin/harness-cli story verify-all`, `_harness/bin/harness-cli done-check`, `_harness/bin/harness-cli evidence`, `evidence` table (migration 007), `story.verify_command`, `story.last_verified_result`, `.github/workflows/harness-cli-release.yml`, `_harness/docs/templates/validation-report.md` | Proof commands run individually or in batch; `story verify` auto-captures a durable evidence log so a proof boolean always has a fresh log; `done-check` packages lane-aware completion gates with an exit code.                       | Benchmark ingestion remains future work.                                                      |
| 9   | Permissions            | Partial | `AGENTS.md`, `_harness/docs/HARNESS.md`, `_harness/docs/FEATURE_INTAKE.md`, `_harness/docs/ARCHITECTURE.md`, installer conflict handling in `scripts/install-harness.sh`                                                                                                                                                                                                                       | Policy describes when agents may update docs and when to ask before architecture or workflow changes.                                                                                                                                | Permissions are instruction-level only; no enforced policy layer or command allowlist exists. |
| 10  | Entropy auditing       | Covered | `_harness/docs/HARNESS_BACKLOG.md`, `_harness/docs/HARNESS_AUDIT.md`, `_harness/docs/IMPROVEMENT_PROTOCOL.md`, `backlog` table, `trace.harness_friction`, `_harness/bin/harness-cli audit`, `_harness/bin/harness-cli propose`, `_harness/docs/HARNESS_MATURITY.md`                                                                                                                                       | Growth rule captures friction, audit detects drift and entropy score, backlog items compare predicted impact to actual outcome, and proposal generation can create reviewable backlog items.                                         | Automated repair remains future work.                                                         |
| 11  | Intervention recording | Covered | `intervention` table, `_harness/bin/harness-cli intervention add`, `_harness/bin/harness-cli query interventions`, `trace` table, `docs/decisions/*`, `docs/stories/*`, `_harness/docs/HARNESS.md`                                                                                                                                                                             | Human, reviewer, CI, and agent interventions are separate durable records and can be filtered by trace, story, or type.                                                                                                              | Capture is still manual and advisory.                                                         |

## NexAU Cross-Reference

| Component            | Harness Equivalent                                                                                                                                                                | Status  | Notes                                                                                                                                           |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| System prompts       | `AGENTS.md` plus Harness policy docs                                                                                                                                              | Covered | `AGENTS.md` is the stable shim; `_harness/docs/HARNESS.md`, `_harness/docs/FEATURE_INTAKE.md`, and `_harness/docs/CONTEXT_RULES.md` carry evolving operating instructions. |
| Tool descriptions    | `_harness/docs/TOOL_REGISTRY.md`, `_harness/docs/scripts-README.md`, `_harness/docs/HARNESS.md`, `_harness/docs/TRACE_SPEC.md`, CLI help from `crates/harness-cli/src/interface.rs`, `_harness/bin/harness-cli query tools` | Covered | Commands are documented in a standalone registry and exposed as compiled plus registered tool manifest entries.                                 |
| Tool implementations | `_harness/bin/harness-cli`, `crates/harness-cli/*`, `_harness/schema/001-init.sql`, `_harness/schema/002-story-verify.sql`                                                           | Covered | The Rust CLI is the primary durable-layer implementation and stable repo-local entrypoint.                                                      |
| Middleware           | installer safety logic, feature intake workflow                                                                                                                                   | Partial | The installer and intake process mediate work, but there is no runtime middleware enforcing policies.                                           |
| Skills               | `_harness/docs/templates/*`, `_harness/docs/FEATURE_INTAKE.md`, `_harness/docs/CONTEXT_RULES.md`, `_harness/docs/TRACE_SPEC.md`                                                                                       | Partial | Reusable procedures exist as markdown, not executable or installable agent skills.                                                              |
| Sub-agents           | None in this repository                                                                                                                                                           | Missing | No delegated specialist agents or sub-agent protocols exist.                                                                                    |
| Long-term memory     | `harness.db`, `docs/decisions/*`, `docs/stories/*`, `_harness/docs/HARNESS_BACKLOG.md`, `_harness/docs/GLOSSARY.md`                                                                                 | Covered | Durable records and markdown decisions preserve task history and project vocabulary.                                                            |

## File Inventory

Every tracked project file plus the Phase 2 input file is mapped to at least one
Runtime Substrate responsibility.

| File                                                                          | Primary Responsibility | Secondary Responsibilities                             |
| ----------------------------------------------------------------------------- | ---------------------- | ------------------------------------------------------ |
| `.gitignore`                                                                  | Tool access            | Task state                                             |
| `AGENTS.md`                                                                   | Context selection      | Task specification, permissions                        |
| `README.md`                                                                   | Task specification     | Project memory                                         |
| `CONTRIBUTING.md`                                                             | Intervention recording | Project memory                                         |
| `Cargo.toml`                                                                  | Tool access            | Verification                                           |
| `Cargo.lock`                                                                  | Tool access            | Verification                                           |
| `PHASE2.md`                                                                   | Task specification     | Observability, context selection                       |
| `PHASE3.md`                                                                   | Task specification     | Observability, verification, entropy auditing          |
| `PHASE4.md`                                                                   | Task specification     | Verification, observability, task state                |
| `PHASE5.md`                                                                   | Task specification     | Verification, entropy auditing, intervention recording |
| `crates/harness-cli/Cargo.toml`                                               | Tool access            | Verification                                           |
| `crates/harness-cli/src/main.rs`                                              | Tool access            | Tool implementation                                    |
| `crates/harness-cli/src/domain.rs`                                            | Tool access            | Task state, verification                               |
| `crates/harness-cli/src/application.rs`                                       | Tool access            | Task state                                             |
| `crates/harness-cli/src/infrastructure.rs`                                    | Tool access            | Project memory, task state, observability              |
| `crates/harness-cli/src/interface.rs`                                         | Tool access            | Context selection, verification                        |
| `_harness/docs/ARCHITECTURE.md`                                                        | Permissions            | Context selection, task specification                  |
| `_harness/docs/FEATURE_INTAKE.md`                                                      | Task specification     | Permissions, context selection                         |
| `_harness/docs/GLOSSARY.md`                                                            | Project memory         | Context selection                                      |
| `_harness/docs/HARNESS.md`                                                             | Task specification     | Project memory, task state, permissions                |
| `_harness/docs/HARNESS_BACKLOG.md`                                                     | Entropy auditing       | Project memory, failure attribution                    |
| `_harness/docs/HARNESS_COMPONENTS.md`                                                  | Failure attribution    | Observability, entropy auditing                        |
| `_harness/docs/HARNESS_MATURITY.md`                                                    | Entropy auditing       | Observability, verification                            |
| `_harness/docs/HARNESS_AUDIT.md`                                                       | Entropy auditing       | Verification, task state                               |
| `_harness/docs/IMPROVEMENT_PROTOCOL.md`                                                | Entropy auditing       | Failure attribution, permissions                       |
| `_harness/docs/CONTEXT_RULES.md`                                                       | Context selection      | Permissions, task specification                        |
| `_harness/docs/TRACE_SPEC.md`                                                          | Observability          | Failure attribution, intervention recording            |
| `_harness/docs/TOOL_REGISTRY.md`                                                       | Tool access            | Context selection, verification                        |
| `_harness/docs/README.md`                                                              | Project memory         | Context selection                                      |
| `_harness/docs/TEST_MATRIX.md`                                                         | Verification           | Task state                                             |
| `docs/decisions/0001-harness-first-development.md`                            | Project memory         | Permissions                                            |
| `docs/decisions/0002-post-spec-product-lifecycle.md`                          | Project memory         | Task specification                                     |
| `docs/decisions/0003-generic-spec-intake-harness.md`                          | Project memory         | Task specification                                     |
| `docs/decisions/0004-sqlite-durable-layer.md`                                 | Project memory         | Observability, task state                              |
| `docs/decisions/0005-prebuilt-rust-harness-cli.md`                            | Project memory         | Tool access                                            |
| `docs/decisions/0006-phase-4-benchmark-triage.md`                             | Project memory         | Verification                                           |
| `docs/decisions/0007-improvement-proposal-rules.md`                           | Project memory         | Entropy auditing, permissions                          |
| `docs/decisions/README.md`                                                    | Project memory         | Context selection                                      |
| `docs/demo/README.md`                                                         | Task specification     | Project memory                                         |
| `docs/product/README.md`                                                      | Task specification     | Project memory                                         |
| `docs/review-fixes-1d30bf62-to-main.md`                                       | Intervention recording | Failure attribution, verification                      |
| `docs/stories/README.md`                                                      | Task specification     | Project memory                                         |
| `docs/stories/US-001-install-harness.md`                                      | Task specification     | Verification, intervention recording                   |
| `docs/stories/US-008-trace-quality-scoring.md`                                | Task specification     | Observability, verification                            |
| `docs/stories/US-009-enriched-friction-query.md`                              | Task specification     | Failure attribution, observability                     |
| `docs/stories/US-011-backlog-outcome-workflow.md`                             | Task specification     | Entropy auditing, project memory                       |
| `docs/stories/US-012-story-verify-command-field.md`                           | Task specification     | Verification                                           |
| `docs/stories/US-015-story-verify-command.md`                                 | Task specification     | Verification                                           |
| `docs/stories/US-016-auto-trace-scoring-on-write.md`                          | Task specification     | Observability, verification                            |
| `docs/stories/US-017-pre-close-verification-gate.md`                          | Task specification     | Verification, permissions                              |
| `docs/stories/US-018-phase4-cli-ux-hardening.md`                              | Task specification     | Tool access, verification                              |
| `docs/stories/US-019-machine-readable-tool-registry.md`                       | Task specification     | Tool access                                            |
| `docs/stories/US-020-batch-story-verification.md`                             | Task specification     | Verification                                           |
| `docs/stories/US-021-intervention-recording-schema.md`                        | Task specification     | Intervention recording                                 |
| `docs/stories/US-022-context-rule-measurement.md`                             | Task specification     | Context selection                                      |
| `docs/stories/US-023-drift-detection-entropy-score.md`                        | Task specification     | Entropy auditing                                       |
| `docs/stories/US-024-improvement-proposal-pipeline.md`                        | Task specification     | Entropy auditing, permissions                          |
| `docs/stories/backlog.md`                                                     | Task specification     | Project memory                                         |
| `docs/stories/epics/README.md`                                                | Task specification     | Project memory                                         |
| `docs/stories/epics/E01-durable-layer/US-002-rust-harness-cli/overview.md`    | Task specification     | Project memory                                         |
| `docs/stories/epics/E01-durable-layer/US-002-rust-harness-cli/design.md`      | Task specification     | Tool access, permissions                               |
| `docs/stories/epics/E01-durable-layer/US-002-rust-harness-cli/execplan.md`    | Task specification     | Verification, task state                               |
| `docs/stories/epics/E01-durable-layer/US-002-rust-harness-cli/validation.md`  | Verification           | Intervention recording                                 |
| `docs/stories/epics/E02-phase-2-observability-taxonomy/phase-2-progress.md`   | Task state             | Intervention recording                                 |
| `docs/stories/epics/E03-phase-5-evolution-infrastructure/phase-5-progress.md` | Task state             | Verification, entropy auditing                         |
| `_harness/docs/templates/decision.md`                                                  | Project memory         | Task specification                                     |
| `_harness/docs/templates/spec-intake.md`                                               | Task specification     | Context selection                                      |
| `_harness/docs/templates/story.md`                                                     | Task specification     | Verification                                           |
| `_harness/docs/templates/validation-report.md`                                         | Verification           | Intervention recording                                 |
| `_harness/docs/templates/high-risk-story/overview.md`                                  | Task specification     | Context selection                                      |
| `_harness/docs/templates/high-risk-story/design.md`                                    | Task specification     | Permissions                                            |
| `_harness/docs/templates/high-risk-story/execplan.md`                                  | Task state             | Verification                                           |
| `_harness/docs/templates/high-risk-story/validation.md`                                | Verification           | Failure attribution                                    |
| `_harness/docs/scripts-README.md`                                                           | Tool access            | Context selection                                      |
| `_harness/bin/harness-cli`                                                     | Tool access            | Task state, observability                              |
| `_harness/bin/harness-cli`                                                     | Tool access            | Task state, observability                              |
| `scripts/install-harness.sh`                                                  | Tool access            | Permissions                                            |
| `scripts/build-harness-cli-release.sh`                                        | Verification           | Tool access                                            |
| `_harness/schema/001-init.sql`                                                 | Task state             | Observability, project memory                          |
| `_harness/schema/002-story-verify.sql`                                         | Verification           | Task state, project memory                             |
| `_harness/schema/003-tool-registry.sql`                                        | Tool access            | Project memory                                         |
| `_harness/schema/004-intervention.sql`                                         | Intervention recording | Failure attribution                                    |
| `_harness/schema/005-tool-extensions.sql`                                      | Tool access            | Verification                                           |
| `_harness/schema/006-next-action.sql`                                          | Task state             | Observability                                          |
| `_harness/schema/007-evidence.sql`                                             | Verification           | Observability, project memory                          |
| `docs/product/read-model.md`                                                  | Observability          | Task specification, verification                       |
| `docs/stories/epics/E02-read-model/*`                                         | Task specification     | Observability, verification                            |
| `docs/decisions/0001-read-model-first-class-conditional.md`                   | Project memory         | Observability                                          |
| `docs/decisions/0002-evidence-gitignore-pointer-store.md`                     | Project memory         | Verification                                           |
| `.github/ISSUE_TEMPLATE/agent-failure-case.md`                                | Failure attribution    | Entropy auditing                                       |
| `.github/ISSUE_TEMPLATE/pattern-request.md`                                   | Entropy auditing       | Intervention recording                                 |
| `.github/ISSUE_TEMPLATE/real-world-example.md`                                | Project memory         | Intervention recording                                 |
| `.github/workflows/harness-cli-release.yml`                                   | Verification           | Tool access                                            |

## Coverage Summary

- Covered: 8/11 responsibilities.
- Partial: 3/11 responsibilities.
- Missing: 0/11 responsibilities.

Covered responsibilities:

- Task specification.
- Context selection.
- Tool access.
- Project memory.
- Task state.
- Verification.
- Entropy auditing.
- Intervention recording. Partial responsibilities:

- Observability.
- Failure attribution.
- Permissions.

Phase 5 converts tool access, entropy auditing, and intervention recording into
covered responsibilities with a registry, drift audit, proposal loop, and
intervention schema. Later phases should focus on benchmark ingestion,
component-level attribution, permission enforcement, and tool usage analytics.
