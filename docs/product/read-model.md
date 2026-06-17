# Read-Model & Evidence Layer

The product contract for the agent-facing **read-path** of the harness: how an
agent (or a fresh session) answers _what has been done / what is in progress /
what needs doing_, with **evidence and artifacts** backing every claim.

Derived from `_harness/docs/proposals/2026-06-17-read-model-evidence-upgrade.md` (frozen
design record after slicing).

## Why this layer exists

The harness was optimized for the **write-path of one task** (intake → story →
trace → proof). "Help the agent understand the project" is a **read-path,
cross-task** problem. There are three reading planes; this layer fills the third
and closes the evidence loop:

| Plane                | Answers                       | Nature             | Status   |
| -------------------- | ----------------------------- | ------------------ | -------- |
| `KNOWLEDGE_INDEX.md` | what the repo _is_            | static, router     | existing |
| `query stats`        | _how many_ (totals)           | dynamic, raw count | existing |
| **Read-Model**       | what is _being / been / to-do_ | dynamic, ranked  | NEW      |

## Invariants (whole layer)

1. **Read-Model is a derived VIEW, not a store.** Every byte `status`/`recap`
   shows must trace back to `intake`/`story`/`trace`/`backlog`/`intervention`.
   `query status` and `query recap` add NO schema.
2. **First-class-conditional.** Orientation reads (`query status`) bind to the
   skip rule in `00-AGENTS.md §3`: a turn that RUNS the 7-stage workflow runs
   them; a one-step Q&A turn skips them. No new classification axis.
3. **Determinism.** `status`, `recap`, `done-check` are deterministic for a
   given db state — no LLM, no randomness, no time dependence beyond existing
   `created_at`.
4. **Token-aware.** `status` targets ≤ ~1k tokens: each section has a line cap
   (default 5) and reports how many were cut ("no silent caps").
5. **Additive migration.** `ALTER TABLE ... ADD COLUMN` for additive change,
   `CREATE TABLE` for new tables, each with `INSERT INTO schema_version`.
6. **Clean repo.** Heavy artifacts live locally and gitignored; the db keeps
   only pointer + hash + digest. Tradeoff accepted: artifacts do not rebuild on
   another machine / CI.

## Capabilities

### Read-Model / Session Brief — `harness-cli query status`

A single ranked snapshot of current project state, ordered by action priority:
in-progress stories, stories needing proof, resume points (partial/blocked work
with next-action), open backlog (high-risk first), recent interventions, recent
activity. Pure read view. Flags: `--json`, `--lane`, `--limit <n>`, `--full`.

### Evidence / Artifact store — `harness-cli evidence add|list`

Durable pointer to local artifacts (logs, diffs, screenshots, reports). The db
stores path + sha256 + bytes + digest + command + result + source; the artifact
file lives under `_harness/evidence/` (gitignored). `story verify` auto-captures
its log by default (keep-last-per-`(story,kind,result)` + sha256 dedup;
`fail→pass` keeps both); `--no-capture` is the draft-run escape hatch.

### Next-action / Resume continuity

`story.next_action` is the live WIP pointer; `trace.next_action` is the
immutable record at trace time. A trace with outcome `partial|blocked|failed`
MUST set `--next-action` (CLI rejects empty); `completed` clears
`story.next_action`. Surfaced in the RESUME section of `query status`.

### Done-check gate — `harness-cli done-check`

A lane-aware aggregator (read + exit code, no new store) packaging existing
checks plus evidence/next-action into one Stage-7 gate. Exit `0` = all pass,
`1` = any fail. Checklist output with per-line reason.

### Recap rollup — `harness-cli query recap`

A deterministic templated rollup (counts/groupings, no semantic summary) over
traces for a story/epic/time window. Saves tokens at Stage 0 and feeds Stage 6
friction review.

## Update Rule

When this layer's behavior changes:

1. Update this product doc.
2. Update the affected story packet under `docs/stories/epics/E02-read-model/`.
3. Update durable proof with `harness-cli story update`.
4. Record a decision if it changes the read-path contract, the workflow stages,
   or the evidence/proof model.
