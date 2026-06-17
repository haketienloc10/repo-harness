# Context Engineering Rules

Context rules help agents decide what to read, when to read it, and when to stop
reading. They are additive to the stable `AGENTS.md` reading list.

The goal is not to maximize context. The goal is to put the right information in
the model for the current task phase and risk lane.

## Context Phases

### Intake Phase

Read to classify the request, find the affected surface, and choose a lane.

| Document Or Source                     | Tiny              | Normal                                              | High-Risk                                |
| -------------------------------------- | ----------------- | --------------------------------------------------- | ---------------------------------------- |
| `docs/KNOWLEDGE_INDEX.md` (read first) | Must              | Must                                                | Must                                     |
| `AGENTS.md`                            | Must              | Must                                                | Must                                     |
| `_harness/docs/FEATURE_INTAKE.md`               | Must              | Must                                                | Must                                     |
| `_harness/bin/harness-cli query matrix` | Must              | Must                                                | Must                                     |
| `README.md`                            | Should            | Must                                                | Must                                     |
| `_harness/docs/HARNESS.md`                      | Should            | Must                                                | Must                                     |
| `_harness/docs/ARCHITECTURE.md`                 | Skip              | Should                                              | Must                                     |
| Relevant `docs/product/*`              | Skip if unrelated | Must if product behavior changes                    | Must                                     |
| Relevant `docs/stories/*`              | Skip if unrelated | Must if a story exists                              | Must                                     |
| `docs/decisions/*`                     | Skip              | Should if architecture or durable rules are touched | Must                                     |
| `_harness/docs/HARNESS_COMPONENTS.md`           | Skip              | Should for Harness improvements                     | Must for observability or benchmark work |

### Planning Phase

Read to decide the smallest safe approach and expected proof.

| Document Or Source                                                    | Tiny   | Normal                              | High-Risk                            |
| --------------------------------------------------------------------- | ------ | ----------------------------------- | ------------------------------------ |
| Current files to edit                                                 | Must   | Must                                | Must                                 |
| `_harness/docs/templates/story.md`                                             | Skip   | Must when creating/updating a story | Should                               |
| `_harness/docs/templates/high-risk-story/*`                                    | Skip   | Skip unless risk escalates          | Must                                 |
| `_harness/docs/ARCHITECTURE.md`                                                | Skip   | Should for code or boundary changes | Must                                 |
| `_harness/docs/TEST_MATRIX.md` or `_harness/bin/harness-cli query matrix`       | Should | Must                                | Must                                 |
| Relevant decisions                                                    | Skip   | Should                              | Must                                 |
| `_harness/docs/HARNESS_MATURITY.md`                                            | Skip   | Should for Harness improvements     | Must for maturity or process changes |
| `_harness/docs/HARNESS_BACKLOG.md` and `_harness/bin/harness-cli query backlog` | Skip   | Should if friction repeats          | Must if changing Harness behavior    |

### Implementation Phase

Read while making the change. Keep this phase scoped to files that directly
affect the selected story.

| Document Or Source                   | Tiny                    | Normal                        | High-Risk                            |
| ------------------------------------ | ----------------------- | ----------------------------- | ------------------------------------ |
| Files being changed                  | Must                    | Must                          | Must                                 |
| Adjacent files with same pattern     | Should                  | Must                          | Must                                 |
| Relevant product docs                | Skip if copy-only       | Must if behavior changes      | Must                                 |
| Relevant story packet                | Skip if no story needed | Must                          | Must                                 |
| Relevant templates                   | Skip                    | Should when adding docs       | Must                                 |
| `_harness/docs/ARCHITECTURE.md`               | Skip                    | Should for structural changes | Must                                 |
| Provider/API/security docs           | Skip                    | Should if touched             | Must                                 |
| Unrelated docs and historical traces | Skip                    | Skip                          | Should only if they affect decisions |

### Validation Phase

Read to prove the change and avoid claiming unsupported completion.

| Document Or Source                                              | Tiny             | Normal                          | High-Risk                                    |
| --------------------------------------------------------------- | ---------------- | ------------------------------- | -------------------------------------------- |
| Story acceptance criteria                                       | Should           | Must                            | Must                                         |
| `_harness/docs/TEST_MATRIX.md` or `_harness/bin/harness-cli query matrix` | Should           | Must                            | Must                                         |
| Validation section of story packet                              | Skip if no story | Must                            | Must                                         |
| `_harness/docs/templates/validation-report.md`                           | Skip             | Should for notable proof        | Must for high-risk proof                     |
| Relevant commands from README/package docs                      | Should           | Must                            | Must                                         |
| Benchmark protocol or external benchmark repo                   | Skip             | Skip unless requested           | Must if the story depends on benchmark proof |
| `_harness/docs/HARNESS_MATURITY.md`                                      | Skip             | Should for Harness improvements | Must for maturity claims                     |

### Trace Phase

Read to leave useful evidence for the next agent and for benchmark scoring.

| Document Or Source                          | Tiny             | Normal                         | High-Risk                             |
| ------------------------------------------- | ---------------- | ------------------------------ | ------------------------------------- |
| `_harness/docs/TRACE_SPEC.md`                        | Should           | Must                           | Must                                  |
| `_harness/bin/harness-cli query matrix`      | Should           | Must                           | Must                                  |
| `_harness/bin/harness-cli query backlog`     | Skip             | Should if friction occurred    | Must                                  |
| Changed-file list from `git status --short` | Must             | Must                           | Must                                  |
| Validation command output                   | Should           | Must                           | Must                                  |
| Story packet or progress log                | Skip if no story | Must                           | Must                                  |
| `_harness/docs/HARNESS_COMPONENTS.md`                | Skip             | Should if attributing friction | Must if failure attribution is needed |

## Retrieval Triggers

| Trigger Condition                                                                              | Action                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Start of any task / onboarding a repo                                                          | Read `docs/KNOWLEDGE_INDEX.md` first as an Orient router (Hierarchy wins on conflict). `harness-cli knowledge check` is a COARSE gate: red ⇒ definitely stale (refresh via `generate-knowledge-index` before relying on it); green ⇒ NOT proof of freshness (it misses subdir changes and stale Purpose/Concepts/descriptions — see `_harness/00-AGENTS.md` §1). |
| Task touches database schema, durable records, or migrations                                   | Read `docs/decisions/0004-sqlite-durable-layer.md`, `_harness/schema/`, and relevant CLI code before planning.                                                                                                                                                                                                                                                    |
| Task touches CLI command behavior or installer distribution                                    | Read `docs/decisions/0005-prebuilt-rust-harness-cli.md`, `_harness/docs/scripts-README.md`, relevant `crates/harness-cli/*` code, CLI help output, and installer docs.                                                                                                                                                                                                         |
| Task touches auth, authorization, audit/security, data loss, or external providers             | Treat as high-risk, read `_harness/docs/templates/high-risk-story/*`, and check prior decisions before implementation.                                                                                                                                                                                                                                                    |
| Task changes public API shape, product behavior, or user-visible workflow                      | Read relevant `docs/product/*`, story packets, and validation expectations before editing.                                                                                                                                                                                                                                                                       |
| Task changes Harness policy, source hierarchy, risk classification, or validation requirements | Read `_harness/docs/HARNESS.md`, `_harness/docs/FEATURE_INTAKE.md`, `_harness/docs/ARCHITECTURE.md`, and `docs/decisions/*`; pause if direction is ambiguous.                                                                                                                                                                                                                               |
| Task discovers repeated confusion, stale docs, or missing proof                                | Read `_harness/docs/HARNESS_BACKLOG.md`, record `harness_friction`, and add a backlog item when the fix is out of scope.                                                                                                                                                                                                                                                  |
| Task makes a maturity, observability, trace quality, or benchmark claim                        | Read `_harness/docs/HARNESS_COMPONENTS.md`, `_harness/docs/HARNESS_MATURITY.md`, and `_harness/docs/TRACE_SPEC.md`.                                                                                                                                                                                                                                                                         |
| Task is normal or high-risk and spans multiple iterations                                      | Create or update a story/progress file under `docs/stories/` and keep it current.                                                                                                                                                                                                                                                                                |
| Final response is being prepared                                                               | Re-read the validation evidence, `git status --short`, and `_harness/docs/TRACE_SPEC.md` before recording the final trace.                                                                                                                                                                                                                                                |

## Token Budget Guidance

| Lane      | Target Context Budget               | Read Shape                                                                                                                                                                | Reasoning                                                                                                        |
| --------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Tiny      | About 2K tokens of Harness context  | `AGENTS.md`, `_harness/docs/FEATURE_INTAKE.md`, matrix query, and the exact file being changed.                                                                                    | Tiny work should not spend more context on policy than on the edit.                                              |
| Normal    | About 5K tokens of Harness context  | Intake docs, relevant product/story docs, architecture when structural, validation expectations, and trace spec at the end.                                               | Normal work needs enough context to preserve contracts and record proof without reading every historical file.   |
| High-risk | About 10K tokens of Harness context | Full intake, architecture, relevant decisions, high-risk templates, product docs, validation docs, trace spec, and component/maturity docs when Harness behavior changes. | High-risk work needs source hierarchy, prior decisions, and proof expectations in context before implementation. |

Budget rules:

- Prefer targeted `rg` searches over bulk reading.
- Read the smallest section that answers the current phase question.
- Escalate context when a retrieval trigger fires.
- Do not keep reading unrelated history after the lane, affected files, and
  validation path are clear.

## Additive Behavior

These rules do not replace `AGENTS.md`. Agents should still read the stable
entrypoint documents listed there before work. This document explains what to
retrieve after that initial context, based on lane, phase, and trigger.

## Review Checklist

Before implementation:

- Lane is chosen from `_harness/docs/FEATURE_INTAKE.md`.
- Relevant product docs or story packets are identified.
- Any high-risk trigger has been handled.

Before final response:

- Validation evidence has been read.
- `_harness/docs/TRACE_SPEC.md` has been read for normal/high-risk tasks.
- The final trace includes files read, files changed, outcome, and friction when
  applicable.
